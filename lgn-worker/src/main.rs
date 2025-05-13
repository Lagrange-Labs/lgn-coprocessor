#![feature(generic_const_exprs)]
#![feature(result_flattening)]
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::io::Write;
use std::panic;
use std::path::Path;
use std::process::ExitCode;
use std::result::Result::Ok;
use std::str::FromStr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::bail;
use anyhow::Context;
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
use lgn_messages::types::ProverType;
use lgn_worker::avs::utils::read_keystore;
use metrics::counter;
use metrics::histogram;
use mimalloc::MiMalloc;
use semver::Version;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::transport::ClientTlsConfig;
use tonic::Request;
use tonic::Streaming;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing::span;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use warp::Filter;

use crate::config::Config;
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

#[derive(Error, Debug)]
enum Error {
    /// The UUID is required.
    #[error("Protobuf message missing required UUID field")]
    UUIDMissing,

    /// The UUID is invalid.
    #[error("Protobuf message UUID is invalid. uuid: {:?}", .uuid)]
    UUIDInvalid { uuid: Vec<u8> },

    /// Failed to parse the incoming envelope.
    #[error("Worker envelope parsing failed. uuid: {} err: {:?} envelope: {:?}", .uuid, .err, .message)]
    EnvelopeParseFailed {
        uuid: uuid::Uuid,
        err: serde_json::Error,
        message: WorkerToGwResponse,
    },

    /// Invalid mp2 version in the incoming envelope.
    #[error("Worker envelope parsing mp2 version failed. uuid: {} err: {:?}", .uuid, .err)]
    EnvelopeInvalidMP2Version {
        uuid: uuid::Uuid,
        err: semver::Error,
    },

    /// Incompatible version required in the incoming envelope.
    #[error("Worker envelope unsupported mp2 version. uuid: {} got: {} requirement: {}", .uuid, .got, .requirement)]
    EnvelopeIncompatibleMP2Version {
        uuid: uuid::Uuid,
        got: semver::Version,
        requirement: semver::VersionReq,
    },

    /// Proof generation returned an error.
    #[error("Worker proof generation failed. uuid: {} err: {:?}", .uuid, .err)]
    ProofFailed {
        uuid: uuid::Uuid,
        err: anyhow::Error,
    },

    /// Proof generation paniced.
    #[error("Worker proof generation paniced. uuid: {} panic: {}", .uuid, .panic_msg)]
    ProofPanic { uuid: uuid::Uuid, panic_msg: String },

    /// Failed to serialise outgoing envelope.
    #[error("Worker reply envelope serialisation failed. uuid: {} err: {:?}", .uuid, .err)]
    ReplySerializationFailed {
        uuid: uuid::Uuid,
        err: serde_json::Error,
    },
}

const ERROR_UUID_MISSING: &str = "uuid_missing";
const ERROR_UUID_INVALID: &str = "uuid_invalid";
const ERROR_ENVELOPE_PARSE_FAILED: &str = "envelope_parse_invalid";
const ERROR_ENVELOPE_INVALID_MP2_VERSION: &str = "envelope_invalid_mp2_version";
const ERROR_ENVELOPE_INCOMPATIBLE_MP2_VERSION: &str = "envelope_incompatbile_mp2_version";
const ERROR_PROOF_INVALID: &str = "proof_invalid";
const ERROR_PROOF_PANIC: &str = "proof_panic";
const ERROR_REPLY_SERIALIZATION_INVALID: &str = "reply_serialization_invalid";

impl Error {
    /// Returns an error tag, suitable to be used for metrics.
    fn to_error_tag(&self) -> &'static str {
        match self {
            Error::UUIDMissing => ERROR_UUID_MISSING,
            Error::UUIDInvalid { .. } => ERROR_UUID_INVALID,
            Error::EnvelopeParseFailed { .. } => ERROR_ENVELOPE_PARSE_FAILED,
            Error::EnvelopeInvalidMP2Version { .. } => ERROR_ENVELOPE_INVALID_MP2_VERSION,
            Error::EnvelopeIncompatibleMP2Version { .. } => ERROR_ENVELOPE_INCOMPATIBLE_MP2_VERSION,
            Error::ProofFailed { .. } => ERROR_PROOF_INVALID,
            Error::ProofPanic { .. } => ERROR_PROOF_PANIC,
            Error::ReplySerializationFailed { .. } => ERROR_REPLY_SERIALIZATION_INVALID,
        }
    }
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
async fn main() -> ExitCode {
    let cli = Cli::parse();
    setup_logging(cli.json);

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

    let config = Config::load(cli.config);
    config.validate();
    debug!("Loaded configuration: {:?}", config);

    if let Err(err) = run(&config).await {
        if let Some(path) = config.exit_reason_path {
            exit_reason(
                &path,
                format!("Worker exited due to an error. err: {:?}", err),
            )
        }
        error!("Worker exited due to an error. err: {:?}", err);
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Saves `reason` to `path`.
///
/// This function is called on termination to report failures, it will ignore
/// all errors since it may not be possible to recover, used to report errors
/// on kubernetes, e.g. `/dev/termination-log` [1].
///
/// [1]: https://kubernetes.io/docs/tasks/debug/debug-application/determine-reason-pod-failure/
pub fn exit_reason(
    path: &str,
    reason: impl AsRef<str>,
) {
    if let Some(parent) = Path::new(path).parent() {
        let _ = fs::create_dir_all(parent);
    }

    // create a new file, or open it if it already exists truncating its contents.
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
    {
        let _ = file.write_all(reason.as_ref().as_bytes());
        let _ = file.sync_data();
    }
}

async fn run(config: &Config) -> anyhow::Result<()> {
    let mp2_version = semver::Version::parse(verifiable_db::version())?;
    let mp2_requirement = semver::VersionReq::parse(&format!("^{}", mp2_version.major))?;
    let version = env!("CARGO_PKG_VERSION");

    let span = span!(
        Level::INFO,
        "run",
        "worker" = config.avs.worker_id.to_string(),
        "issuer" = config.avs.issuer.to_string(),
        "version" = version,
        "class" = config.worker.instance_type.to_string(),
    );
    let _guard = span.enter();

    info!(
        "Starting worker. version: {} mp2_version: {} mp2_requirement: {}",
        version, mp2_version, mp2_requirement
    );

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.prometheus.port))
        .install()
        .context("setting up Prometheus")?;

    let checksums = if cfg!(not(feature = "dummy-prover")) {
        fetch_checksums(config.public_params.checksum_file_url())
            .await
            .context("downloading checksum file")?
    } else {
        Default::default()
    };

    let mut provers_manager = ProversManager::new(config, &checksums).await?;

    // Connect to the GW
    let (mut inbound, outbound) = connect_to_gateway(config, version, &mp2_version).await?;

    let liveness_check_interval = config.worker.liveness_check_interval;
    let last_task_processed =
        AtomicU64::new(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
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

    // Initialise the metrics early on for better dashboards
    counter!("zkmr_worker_messages_total").increment(0);
    counter!("zkmr_worker_messages_successful_total").increment(0);
    for error_tag in [
        ERROR_UUID_MISSING,
        ERROR_UUID_INVALID,
        ERROR_ENVELOPE_PARSE_FAILED,
        ERROR_ENVELOPE_INVALID_MP2_VERSION,
        ERROR_ENVELOPE_INCOMPATIBLE_MP2_VERSION,
        ERROR_PROOF_INVALID,
        ERROR_PROOF_PANIC,
        ERROR_REPLY_SERIALIZATION_INVALID,
    ] {
        counter!("zkmr_worker_messages_error_total", "type" => error_tag).increment(0);
    }

    for task_type in [
        "mapping_leaf",
        "mapping_branch",
        "multi_var_leaf",
        "multi_var_branch",
        "length",
        "contract",
        "block",
        "final_extraction",
        "final_extraction_lengthed",
        "final_extraction_merge",
        "offchain",
        "cell_leaf",
        "cell_partial",
        "cell_full",
        "row_leaf",
        "row_partial",
        "row_full",
        "index",
        "ivc",
    ] {
        counter!(
            "zkmr_worker_tasks_received_total",
            "task_type" => task_type.to_string(),
        )
        .increment(0);
        counter!(
            "zkmr_worker_tasks_successful_total",
            "task_type" => task_type.to_string(),
        )
        .increment(0);
        counter!(
            "zkmr_worker_tasks_error_total",
            "task_type" => task_type.to_string(),
        )
        .increment(0);
    }

    loop {
        debug!("Waiting for message");

        match inbound.next().await {
            Some(Ok(msg)) => {
                counter!("zkmr_worker_messages_total").increment(1);
                let task_id = msg.task_id.clone();

                let outbound_msg = match process_message_from_gateway(
                    &mut provers_manager,
                    msg,
                    &mp2_requirement,
                )
                .await
                {
                    Ok(serialised_reply) => {
                        counter!("zkmr_worker_messages_successful_total").increment(1);
                        WorkerToGwRequest {
                            request: Some(lagrange::worker_to_gw_request::Request::WorkerDone(
                                WorkerDone {
                                    task_id,
                                    reply: Some(Reply::TaskOutput(serialised_reply)),
                                },
                            )),
                        }
                    },
                    Err(err) => {
                        error!("Processing message failed. err: {:?}", err);
                        counter!("zkmr_worker_messages_error_total", "type" => err.to_error_tag())
                            .increment(1);
                        WorkerToGwRequest {
                            request: Some(lagrange::worker_to_gw_request::Request::WorkerDone(
                                WorkerDone {
                                    task_id,
                                    reply: Some(Reply::WorkerError(format!("{:?}", err))),
                                },
                            )),
                        }
                    },
                };

                outbound.send(outbound_msg).await?;

                last_task_processed.store(
                    SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
                    Ordering::Relaxed,
                );
            },
            Some(Err(status)) => {
                counter!("zkmr_worker_error_total").increment(1);
                bail!("connection to the gateway ended. status: {}", status);
            },
            None => {
                bail!("inbound connection broken");
            },
        }
    }
}

/// Connects to the gateway and returns the communication streams.
///
/// The returned tuple has two streams. The first stream is for the gateway's responses, which in
/// this situation are tasks sent from the GW to the worker to be processed. The second stream is
/// the worker requests, which contains the results of the tasks (either a proof or an error).
///
/// The nomeclature is inverted because it is written from the perspective of the gateway, and the
/// client is automatically generated via tonic.
async fn connect_to_gateway(
    config: &Config,
    version: &str,
    mp2_version: &Version,
) -> anyhow::Result<(
    Streaming<WorkerToGwResponse>,
    mpsc::Sender<WorkerToGwRequest>,
)> {
    // Drop the wallet as soon as possible
    let token = {
        let wallet = get_wallet(config)?;
        let claims = get_claims(config)?;
        JWTAuth::new(claims, &wallet)?.encode()?
    };

    let grpc_url = &config.avs.gateway_url;
    let max_message_size = config
        .avs
        .max_grpc_message_size_mb
        .unwrap_or(MAX_GRPC_MESSAGE_SIZE_MB)
        * 1024
        * 1024;

    let uri = grpc_url
        .parse::<tonic::transport::Uri>()
        .context("parsing gateway URL")?;

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    info!(
        "Connecting to the gateway. grpc_url: {}, max_message_size: {}MB",
        grpc_url,
        max_message_size / (1024 * 1024)
    );

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

    let (outbound, outbound_rx) = mpsc::channel(50);
    let outbound_rx = tokio_stream::wrappers::ReceiverStream::new(outbound_rx);
    outbound
        .send(WorkerToGwRequest {
            request: Some(lagrange::worker_to_gw_request::Request::WorkerReady(
                lagrange::WorkerReady {
                    version: version.to_string(),
                    worker_class: format!("{}-{}", config.worker.instance_type, mp2_version.major),
                },
            )),
        })
        .await?;

    let response = client
        .worker_to_gw(tonic::Request::new(outbound_rx))
        .await
        .context("connecting worker_to_gw")?;

    info!("Bidirectional stream with GW opened");
    let inbound = response.into_inner();

    Ok((inbound, outbound))
}

/// Parses, validated, and proves a task.
///
/// # Errors
///
/// - When the message is invalid or unsupported
/// - When the proof inputs are invalid and the prover panics
/// - When the message contents are inconsistent
///
/// See [Error] for details.
async fn process_message_from_gateway(
    provers_manager: &mut ProversManager,
    message: WorkerToGwResponse,
    mp2_requirement: &semver::VersionReq,
) -> Result<Vec<u8>, Error> {
    let uuid = message.task_id.as_ref().ok_or(Error::UUIDMissing)?;

    let uuid = uuid
        .id
        .clone()
        .try_into()
        .map_err(|uuid| Error::UUIDInvalid { uuid })?;
    let uuid = uuid::Uuid::from_bytes_le(uuid);

    let envelope = serde_json::from_slice::<MessageEnvelope>(&message.task)
        .map_err(|err| Error::EnvelopeParseFailed { uuid, err, message })?;

    let envelope_version = semver::Version::parse(&envelope.version)
        .map_err(|err| Error::EnvelopeInvalidMP2Version { uuid, err })?;

    let span = span!(
        Level::INFO,
        "msg",
        %uuid,
        task_id = envelope.task_id,
        query_id = envelope.query_id,
        db_id = ?envelope.db_task_id,
    );
    let _guard = span.enter();

    if !mp2_requirement.matches(&envelope_version) {
        return Err(Error::EnvelopeIncompatibleMP2Version {
            uuid,
            got: envelope_version,
            requirement: mp2_requirement.clone(),
        });
    };

    let task_type = envelope.to_task_type().to_owned();
    let task_id = envelope.task_id().to_string();
    let query_id = envelope.query_id().to_string();

    info!(
        "Received Task. uuid: {} task_id: {} query_id: {}",
        uuid, task_id, query_id
    );

    counter!(
        "zkmr_worker_tasks_received_total",
        "task_type" => task_type.clone(),
    )
    .increment(1);

    let start_time = std::time::Instant::now();
    let result = tokio::task::block_in_place(move || -> Result<MessageReplyEnvelope, Error> {
        // Plonky2 circuits will panic on invalid inputs. Catch these errors and report it to the
        // gateway.
        std::panic::catch_unwind(|| {
            provers_manager
                .delegate_proving(envelope)
                .map_err(|err| Error::ProofFailed { uuid, err })
        })
        .map_err(|panic| {
            if let Some(str) = panic.downcast_ref::<&'static str>() {
                let panic_msg = (*str).to_owned();
                return Error::ProofPanic { uuid, panic_msg };
            }

            match panic.downcast::<String>() {
                Ok(panic_msg) => {
                    Error::ProofPanic {
                        uuid,
                        panic_msg: *panic_msg,
                    }
                },
                Err(panic) => {
                    Error::ProofPanic {
                        uuid,
                        panic_msg: format!("{:?}", panic),
                    }
                },
            }
        })
        .flatten()
    });

    match result {
        Ok(reply) => {
            counter!(
                "zkmr_worker_tasks_successful_total",
                "task_type" => task_type.clone(),
            )
            .increment(1);
            histogram!(
                "zkmr_worker_task_sucessful_processing_duration_seconds",
                "task_type" => task_type.clone(),
            )
            .record(start_time.elapsed().as_secs_f64());

            let serialised = serde_json::to_vec(&reply)
                .map_err(|err| Error::ReplySerializationFailed { uuid, err })?;
            histogram!(
                "zkmr_worker_reply_size_bytes",
                "task_type" => task_type.clone(),
            )
            .record(serialised.len() as f64);

            info!(
                "Processed task. uuid: {} task_id: {} query_id: {} time: {:?}",
                uuid,
                task_id,
                query_id,
                start_time.elapsed(),
            );
            Ok(serialised)
        },
        Err(err) => {
            counter!(
                "zkmr_worker_tasks_error_total",
                "task_type" => task_type.clone(),
            )
            .increment(1);
            histogram!(
                "zkmr_worker_task_failed_processing_duration_seconds",
                "task_type" => task_type.clone(),
            )
            .record(start_time.elapsed().as_secs_f64());

            error!(
                "Failed to process task. uuid: {} task_id: {} query_id: {} time: {:?} err: {:?}",
                uuid,
                task_id,
                query_id,
                start_time.elapsed(),
                err,
            );

            Err(err)
        },
    }
}

/// Build the node's wallet from the configuration file.
fn get_wallet(config: &Config) -> anyhow::Result<Wallet<SigningKey>> {
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

/// Build the JWT claims from the configuration file.
fn get_claims(config: &Config) -> anyhow::Result<Claims> {
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
