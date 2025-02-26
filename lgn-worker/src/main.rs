use std::collections::BTreeMap;
use std::fmt::Debug;
use std::panic;
use std::result::Result::Ok;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
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
use tonic::transport::ClientTlsConfig;
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
use warp::Filter;

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

    let last_task_processed =
        AtomicU64::new(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());

    if let Err(err) = run(cli, mp2_requirement, last_task_processed).await {
        panic!("Worker exited due to an error: {err:?}")
    } else {
        Ok(())
    }
}

async fn run(
    cli: Cli,
    mp2_requirement: semver::VersionReq,
    last_task_processed: AtomicU64,
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

    run_worker(&config, mp2_requirement, last_task_processed).await
}

async fn run_worker(
    config: &Config,
    mp2_requirement: semver::VersionReq,
    last_task_processed: AtomicU64,
) -> Result<()> {
    let max_message_size = config
        .avs
        .max_grpc_message_size_mb
        .unwrap_or(MAX_GRPC_MESSAGE_SIZE_MB)
        * 1024
        * 1024;

    // Preparing the prover
    let checksums = fetch_checksums(config.public_params.checksum_file_url())
        .await
        .context("downloading checksum file")?;
    let mut provers_manager =
        tokio::task::block_in_place(move || -> Result<ProversManager<TaskType, ReplyType>> {
            let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
            register_v1_provers(config, &mut provers_manager, &checksums)
                .context("while registering provers")?;
            Ok(provers_manager)
        })
        .context("creating prover managers")?;

    // Connecting to the GW
    let wallet = get_wallet(config).context("fetching wallet")?;
    let claims = get_claims(config).context("building claims")?;
    let token = JWTAuth::new(claims, &wallet)?.encode()?;

    let grpc_url = &config.avs.gateway_url;
    info!(
        "connecting to the gateway: {}, max. mess. size = {}MB",
        grpc_url,
        max_message_size / (1024 * 1024)
    );

    let uri = grpc_url
        .parse::<tonic::transport::Uri>()
        .context("parsing gateway URL")?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let channel = tonic::transport::Channel::builder(uri.clone())
        .tls_config(ClientTlsConfig::new().with_enabled_roots())?
        .connect()
        .await
        .with_context(|| format!("creating transport channel builder for {uri}"))?;
    let token: MetadataValue<_> = format!("Bearer {token}").parse()?;
    let mut client = lagrange::workers_service_client::WorkersServiceClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut().insert("authorization", token.clone());
            Ok(req)
        },
    )
    .max_encoding_message_size(max_message_size)
    .max_decoding_message_size(max_message_size);

    let (mut outbound, outbound_rx) = tokio::sync::mpsc::channel(50);
    let outbound_rx = tokio_stream::wrappers::ReceiverStream::new(outbound_rx);
    outbound
        .send(WorkerToGwRequest {
            request: Some(lagrange::worker_to_gw_request::Request::WorkerReady(
                lagrange::WorkerReady {
                    version: env!("CARGO_PKG_VERSION").to_string(),
                    worker_class: format!(
                        "{}-{}",
                        config.worker.instance_type,
                        semver::Version::parse(verifiable_db::version())
                            .unwrap()
                            .major
                    ),
                },
            )),
        })
        .await?;

    let response = client
        .worker_to_gw(tonic::Request::new(outbound_rx))
        .await
        .context("connecting `worker_to_gw`")?;

    info!("Bidirectional stream with GW opened");
    let mut inbound = response.into_inner();

    let liveness_check_interval = config.worker.liveness_check_interval;
    let last_task_processed = Arc::new(last_task_processed);
    let last_task_processed_clone = Arc::clone(&last_task_processed);

    // Start readiness and liveness check server
    tokio::spawn(async move {
        let readiness_route = warp::path!("readiness")
            .map(|| warp::reply::with_status("OK", warp::http::StatusCode::OK));
        let liveness_route = warp::path!("liveness").map(move || {
            let last_processed = last_task_processed_clone.load(Ordering::Relaxed);
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now - last_processed <= liveness_check_interval {
                warp::reply::with_status("OK", warp::http::StatusCode::OK)
            } else {
                warp::reply::with_status("FAIL", warp::http::StatusCode::INTERNAL_SERVER_ERROR)
            }
        });
        let routes = readiness_route.or(liveness_route);
        warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
    });

    loop {
        debug!("Waiting for message...");
        tokio::select! {
            Some(inbound_message) = inbound.next() => {
                let msg = match inbound_message {
                    Ok(ref msg) => msg,
                    Err(e) => {
                        bail!("connection to the gateway ended with status: {e}");
                    }
                };
                let result = process_message_from_gateway(&mut provers_manager, msg, &mut outbound, &mp2_requirement).await;
                if result.is_ok() {
                    last_task_processed.store(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(), Ordering::Relaxed);
                }
                if let Err(e) = result {
                    bail!("task processing failed: {e:?}");
                }
            }
            else => {
                bail!("inbound connection broken");
            },
        }
    }
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
    let uuid = message
        .task_id
        .as_ref()
        .map(|id| uuid::Uuid::from_bytes_le(id.id.clone().try_into().unwrap()).to_string())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let reply = {
        let uuid = uuid.clone();
        tokio::task::block_in_place(move || -> Result<MessageReplyEnvelope<ReplyType>, String> {
            serde_json::from_slice::<MessageEnvelope<TaskType>>(&message.task)
                .map_err(|e| {
                    format!(
                        "failed to deserialize envelope for task {} ({}B): {e}",
                        uuid,
                        message.task.len(),
                    )
                })
                .and_then(|message_envelope| {
                    info!("processing task {}", message_envelope.id());
                    process_downstream_payload(provers_manager, message_envelope, mp2_requirement)
                })
        })
    };

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
            tracing::error!("failed to process task {uuid}: {error_str}");
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
