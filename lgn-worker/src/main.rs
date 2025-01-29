use std::collections::BTreeMap;
use std::fmt::Debug;
use std::panic;
use std::result::Result::Ok;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::*;
use backtrace::Backtrace;
use checksum::fetch_checksums;
use clap::Parser;
use ethers::signers::Wallet;
use jwt::Claims;
use jwt::RegisteredClaims;
use k256::ecdsa::SigningKey;
use lagrange::worker_done::Reply;
use lagrange::WorkerDone;
use lagrange::WorkerToGwRequest;
use lagrange::WorkerToGwResponse;
use lgn_auth::jwt::JWTAuth;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_worker::avs::utils::read_keystore;
use metrics::counter;
use mimalloc::MiMalloc;
use tokio_stream::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing::span;
use tracing::trace;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::manager::v1::register_v1_provers;
use crate::manager::ProversManager;

pub mod lagrange {
    tonic::include_proto!("lagrange");
}

mod checksum;
mod config;
mod manager;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const MAX_GRPC_MESSAGE_SIZE_MB: usize = 16;

#[derive(Parser, Clone, Debug)]
struct Cli {
    /// Path to the configuration file.
    #[clap(short, long)]
    config: Option<String>,

    /// If set, output logs in JSON format.
    #[clap(short, long, action)]
    json: bool,
}

fn setup_logging(json: bool) {
    if json {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_level(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Setting up logging failed");
    } else {
        let subscriber = tracing_subscriber::fmt()
            .pretty()
            .compact()
            .with_level(true)
            .with_file(true)
            .with_line_number(true)
            .with_target(true)
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("Setting up logging failed");
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    setup_logging(cli.json);

    let mp2_version = semver::Version::parse(verifiable_db::version())?;
    let mp2_requirement = semver::VersionReq::parse(&format!("^{mp2_version}"))?;

    info!("Running MR2 version {mp2_version} - requiring {mp2_requirement}");

    panic::set_hook(Box::new(|panic_info| {
        let msg = match panic_info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match panic_info.payload().downcast_ref::<String>() {
                    Some(s) => &s[..],
                    None => "Box<dyn Any>",
                }
            },
        };
        let (file, lineno, col) = match panic_info.location() {
            Some(l) => (l.file(), l.line(), l.column()),
            None => ("<unknown>", 0, 0),
        };

        error!(
            msg,
            file,
            lineno,
            col,
            "Panic occurred: {:?}",
            Backtrace::new(),
        );
    }));

    if let Err(err) = run(cli, mp2_requirement).await {
        error!("{err:?}");
        bail!("Worker exited due to an error")
    } else {
        Ok(())
    }
}

async fn run(
    cli: Cli,
    mp2_requirement: semver::VersionReq,
) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    info!("Starting worker. version: {}", version);
    let config = Config::load(cli.config);
    config.validate();
    debug!("Loaded configuration: {:?}", config);

    let span = span!(
        Level::INFO,
        "Starting node",
        "worker" = config.avs.worker_id.to_string(),
        "issuer" = config.avs.issuer.to_string(),
        "version" = version,
        "class" = config.worker.instance_type.to_string(),
    );
    let _guard = span.enter();

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.prometheus.port))
        .install()
        .context("setting up Prometheus")?;

    run_worker(&config, mp2_requirement).await
}

async fn run_worker(
    config: &Config,
    mp2_requirement: semver::VersionReq,
) -> Result<()> {
    let grpc_url = &config.avs.gateway_url;
    info!("Connecting to the gateway: {}", grpc_url);

    let uri = grpc_url
        .parse::<tonic::transport::Uri>()
        .context("parsing gateway URL")?;
    let (mut outbound, outbound_rx) = tokio::sync::mpsc::channel(1024);

    let checksums = fetch_checksums(config.public_params.checksum_file_url())?;
    let mut provers_manager =
        tokio::task::block_in_place(move || -> Result<ProversManager<TaskType, ReplyType>> {
            let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
            register_v1_provers(config, &mut provers_manager, &checksums)
                .context("while registering provers")?;
            Ok(provers_manager)
        })?;

    let outbound_rx = tokio_stream::wrappers::ReceiverStream::new(outbound_rx);

    let wallet = get_wallet(config)?;
    let claims = get_claims(config)?;
    let token = JWTAuth::new(claims, &wallet)?.encode()?;

    let channel = tonic::transport::Channel::builder(uri).connect().await?;
    let token: MetadataValue<_> = format!("Bearer {token}").parse()?;

    let max_message_size = config
        .avs
        .max_grpc_message_size_mb
        .unwrap_or(MAX_GRPC_MESSAGE_SIZE_MB)
        * 1024
        * 1024;

    let mut client = lagrange::workers_service_client::WorkersServiceClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        },
    )
    .max_decoding_message_size(max_message_size)
    .max_encoding_message_size(max_message_size);

    let response = client
        .worker_to_gw(tonic::Request::new(outbound_rx))
        .await?;

    let mut inbound = response.into_inner();

    outbound
        .send(WorkerToGwRequest {
            request: Some(lagrange::worker_to_gw_request::Request::WorkerReady(
                lagrange::WorkerReady {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    worker_class: config.worker.instance_type.to_string(),
                },
            )),
        })
        .await?;

    loop {
        tokio::select! {
            Some(inbound_message) = inbound.next() => {
                let msg = match inbound_message {
                    Ok(ref msg) => msg,
                    Err(e) => {
                        error!("connection to the gateway ended with status: {e}");
                        break;
                    }
                };
                process_message_from_gateway(&mut provers_manager, msg, &mut outbound, &mp2_requirement).await?;
            }
            else => break,
        }
    }

    Ok(())
}

fn process_downstream_payload(
    provers_manager: &ProversManager<TaskType, ReplyType>,
    envelope: MessageEnvelope<TaskType>,
    mp2_requirement: &semver::VersionReq,
) -> Result<MessageReplyEnvelope<ReplyType>, String> {
    let span = span!(
        Level::INFO,
        "Received Task",
        "query_id" = envelope.query_id,
        "task_id" = envelope.task_id,
        "db_id" = ?envelope.db_task_id,
    );
    let _guard = span.enter();

    trace!("Received task. envelope: {:?}", envelope);
    counter!("zkmr_worker_tasks_received_total").increment(1);

    let envelope_version = semver::Version::parse(&envelope.version)
        .context("parsing message version")
        .map_err(|e| e.to_string())?;

    if !mp2_requirement.matches(&envelope_version) {
        return Err(format!(
            "version mismatch: worker requires {mp2_requirement}, task = {envelope_version}"
        ));
    }

    match std::panic::catch_unwind(|| provers_manager.delegate_proving(&envelope)) {
        Ok(result) => {
            match result {
                Ok(reply) => {
                    trace!("Sending reply: {:?}", reply);
                    counter!("zkmr_worker_tasks_processed_total").increment(1);
                    Ok(reply)
                },
                Err(e) => {
                    error!("Error processing task: {:?}", e);
                    counter!("zkmr_worker_error_count", "error_type" =>  "proof processing")
                        .increment(1);

                    Err(format!("{e:?}"))
                },
            }
        },
        Err(panic) => {
            counter!(
                "zkmr_worker_error_count",
                "error_type" => "proof_processing"
            )
            .increment(1);

            let msg = match panic.downcast_ref::<&'static str>() {
                Some(s) => *s,
                None => {
                    match panic.downcast_ref::<String>() {
                        Some(s) => &s[..],
                        None => "Box<dyn Any>",
                    }
                },
            };

            error!("panic encountered while proving {} : {msg}", envelope.id());
            Err(format!("{}: {msg}", envelope.id()))
        },
    }
}

async fn process_message_from_gateway(
    provers_manager: &mut ProversManager<TaskType, ReplyType>,
    message: &WorkerToGwResponse,
    outbound: &mut tokio::sync::mpsc::Sender<WorkerToGwRequest>,
    mp2_requirement: &semver::VersionReq,
) -> Result<()> {
    let message_envelope = serde_json::from_slice::<MessageEnvelope<TaskType>>(&message.task)?;
    info!("processing task {}", message_envelope.id());

    let reply =
        tokio::task::block_in_place(move || -> Result<MessageReplyEnvelope<ReplyType>, String> {
            process_downstream_payload(provers_manager, message_envelope, mp2_requirement)
        });

    let outbound_msg = match reply {
        Ok(reply) => {
            WorkerToGwRequest {
                request: Some(lagrange::worker_to_gw_request::Request::WorkerDone(
                    WorkerDone {
                        task_id: message.task_id.clone(),
                        reply: Some(Reply::TaskOutput(serde_json::to_vec(&reply)?)),
                    },
                )),
            }
        },
        Err(error_str) => {
            WorkerToGwRequest {
                request: Some(lagrange::worker_to_gw_request::Request::WorkerDone(
                    WorkerDone {
                        task_id: message.task_id.clone(),
                        reply: Some(Reply::WorkerError(error_str)),
                    },
                )),
            }
        },
    };
    outbound.send(outbound_msg).await?;

    counter!("zkmr_worker_grpc_messages_sent_total",
                                    "message_type" => "text")
    .increment(1);
    Ok(())
}

fn get_wallet(config: &Config) -> Result<Wallet<SigningKey>> {
    let res = match (
        &config.avs.lagr_keystore,
        &config.avs.lagr_pwd,
        &config.avs.lagr_private_key,
    ) {
        (Some(keystore_path), Some(password), None) => {
            read_keystore(keystore_path, password.expose_secret())?
        },
        (Some(_), None, Some(pkey)) => {
            Wallet::from_str(pkey.expose_secret()).context("Failed to create wallet")?
        },
        _ => bail!("Must specify either keystore path w/ password OR private key"),
    };

    Ok(res)
}

fn get_claims(config: &Config) -> Result<Claims> {
    let registered = RegisteredClaims {
        issuer: Some(config.avs.issuer.clone()),
        subject: Some(config.avs.worker_id.clone()),
        issued_at: Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Epoch can not be in the future")
                .as_secs(),
        ),
        ..Default::default()
    };

    let private = [(
        "worker_class".to_string(),
        serde_json::Value::String(config.worker.instance_type.to_string()),
    )]
    .into_iter()
    .collect::<BTreeMap<String, serde_json::Value>>();

    Ok(Claims {
        registered,
        private,
    })
}
