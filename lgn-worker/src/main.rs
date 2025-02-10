use std::collections::BTreeMap;
use std::fmt::Debug;
use std::net::TcpStream;
use std::panic;
use std::result::Result::Ok;
use std::str::FromStr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::*;
use backtrace::Backtrace;
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
use lgn_messages::types::DownstreamPayload;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::UpstreamPayload;
use lgn_worker::avs::utils::read_keystore;
use metrics::counter;
use mimalloc::MiMalloc;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};
use tokio_stream::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tonic::transport::{Channel, Error};
use tonic::transport::Error as TonicError;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing::span;
use tracing::trace;
use tracing::warn;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use tungstenite::connect;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::Message;
use tungstenite::WebSocket;

use crate::checksum::fetch_checksum_file;
use crate::checksum::verify_directory_checksums;
use crate::config::Config;
use crate::manager::v1::register_v1_provers;
use crate::manager::ProversManager;

pub mod lagrange
{
    tonic::include_proto!("lagrange");
}

mod checksum;
mod config;
mod manager;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

const MAX_GRPC_MESSAGE_SIZE_MB: usize = 16;

#[derive(Parser, Clone, Debug)]
struct Cli
{
    /// Path to the configuration file.
    #[clap(
        short,
        long
    )]
    config: Option<String>,

    /// If set, output logs in JSON format.
    #[clap(
        short,
        long,
        action
    )]
    json: bool,
}

fn setup_logging(json: bool)
{
    if json
    {
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
    }
    else
    {
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
async fn main() -> anyhow::Result<()>
{
    let cli = Cli::parse();
    setup_logging(cli.json);

    panic::set_hook(
        Box::new(
            |panic_info| {
                let msg = match panic_info
                    .payload()
                    .downcast_ref::<&'static str>()
                {
                    Some(s) => *s,
                    None =>
                    {
                        match panic_info
                            .payload()
                            .downcast_ref::<String>()
                        {
                            Some(s) => &s[..],
                            None => "Box<dyn Any>",
                        }
                    },
                };
                let (file, lineno, col) = match panic_info.location()
                {
                    Some(l) =>
                    {
                        (
                            l.file(),
                            l.line(),
                            l.column(),
                        )
                    },
                    None =>
                    {
                        (
                            "<unknown>",
                            0,
                            0,
                        )
                    },
                };

                error!(
                    msg,
                    file,
                    lineno,
                    col,
                    "Panic occurred: {:?}",
                    Backtrace::new(),
                );
            },
        ),
    );

    if let Err(err) = run(cli).await
    {
        error!(
            "Service exiting with an error. err: {:?}",
            err
        );
        bail!("Worker exited due to an error")
    }
    else
    {
        Ok(())
    }
}

async fn run(cli: Cli) -> Result<()>
{
    let version = env!("CARGO_PKG_VERSION");
    info!(
        "Starting worker. version: {}",
        version
    );

    let config = Config::load(cli.config);
    config.validate();
    debug!(
        "Loaded configuration: {:?}",
        config
    );

    let span = span!(
        Level::INFO,
        "Starting node",
        "worker" = config
            .avs
            .worker_id
            .to_string(),
        "issuer" = config
            .avs
            .issuer
            .to_string(),
        "version" = version,
        "class" = config
            .worker
            .instance_type
            .to_string(),
    );
    let _guard = span.enter();

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(
            (
                [
                    0,
                    0,
                    0,
                    0,
                ],
                config
                    .prometheus
                    .port,
            ),
        )
        .install()?;

    if let Some(grpc_url) = &config
        .avs
        .gateway_grpc_url
    {
        run_with_grpc(
            &config,
            grpc_url,
        )
        .await
    }
    else
    {
        tokio::task::block_in_place(move || run_with_websocket(&config))
    }
}

async fn maybe_verify_checksums(config: &Config) -> Result<()>
{
    if config
        .public_params
        .skip_checksum
    {
        return Ok(());
    }

    let checksum_url = &config
        .public_params
        .checksum_file_url();
    let expected_checksums_file = &config
        .public_params
        .checksum_expected_local_path;

    let response = reqwest::get(checksum_url)
        .await
        .context("Failed to fetch checksum file")?
        .text()
        .await
        .context("Failed to read response text")?;

    tokio::fs::File::create(expected_checksums_file)
        .await
        .context("Failed to create local checksum file")?
        .write_all(response.as_bytes())
        .await
        .context("Failed to write checksum file")?;

    verify_directory_checksums(
        &config
            .public_params
            .dir,
        &config
            .public_params
            .checksum_expected_local_path,
    )
    .context("Failed to verify checksums")
}

async fn run_with_grpc(
    config: &Config,
    grpc_url: &str,
) -> Result<()>
{
    let uri = grpc_url.parse::<tonic::transport::Uri>()?;
    let (mut outbound, outbound_rx) = tokio::sync::mpsc::channel(1024);

    let mut provers_manager = tokio::task::block_in_place(
        move || -> Result<ProversManager<TaskType, ReplyType>> {
            let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
            register_v1_provers(
                config,
                &mut provers_manager,
            )
            .context("while registering provers")?;
            Ok(provers_manager)
        },
    )?;

    maybe_verify_checksums(config).await?;

    info!("Connecting to the gateway using gRPC. grpc_url: {}", grpc_url);
    let outbound_rx = tokio_stream::wrappers::ReceiverStream::new(outbound_rx);

    let wallet = get_wallet(config)?;
    let claims = get_claims(config)?;
    let token = JWTAuth::new(
        claims,
        &wallet,
    )?
    .encode()?;
    let token: MetadataValue<_> = format!("Bearer {token}").parse()?;

    let max_message_size = config
        .avs
        .max_grpc_message_size_mb
        .unwrap_or(MAX_GRPC_MESSAGE_SIZE_MB)
        * 1024
        * 1024;

    let max_retries = 5;
    let mut attempt = 0;
    let mut delay = Duration::from_secs(1);

    let channel = loop {
        match Channel::builder(uri.clone()).connect().await {
            Ok(channel) => break channel,
            Err(e) => {
                attempt += 1;
                if is_connection_issue(&e) {
                    warn!("Server closed the connection. Retrying...");
                }
                if attempt >= max_retries {
                    error!("Failed to connect after {} attempts: {}", max_retries, e);
                    return Err(e.into());
                }
                warn!(
                    "Failed to connect (attempt {}/{}): {}. Retrying in {:?}...",
                    attempt, max_retries, e, delay
                );
                sleep(delay).await;
                delay *= 2; // Exponential backoff
            }
        }
    };

    let mut client = lagrange::workers_service_client::WorkersServiceClient::with_interceptor(
        channel,
        move |mut req: Request<()>| {
            req.metadata_mut()
                .insert(
                    "authorization",
                    token.clone(),
                );
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
        .send(
            WorkerToGwRequest {
                request: Some(
                    lagrange::worker_to_gw_request::Request::WorkerReady(
                        lagrange::WorkerReady {
                            version: env!("CARGO_PKG_VERSION").to_string(),
                            worker_class: config
                                .worker
                                .instance_type
                                .to_string(),
                        },
                    ),
                ),
            },
        )
        .await?;

    loop
    {
        tokio::select! {
            Some(inbound_message) = inbound.next() => {
                let msg = match inbound_message {
                    Ok(ref msg) => msg,
                    Err(e) => {
                        error!("connection to the gateway ended with status: {e}");
                        break;
                    }
                };
                process_message_from_gateway(&mut provers_manager, msg, &mut outbound).await?;
            }
            else => break,
        }
    }

    Ok(())
}

fn is_connection_issue(e: &TonicError) -> bool {
    match e {
        TonicError::Connect(_) => true,
        TonicError::Grpc(status) if status.code() == tonic::Code::Unavailable => true,
        _ => false,
    }
}

fn process_downstream_payload(
    provers_manager: &ProversManager<TaskType, ReplyType>,
    envelope: MessageEnvelope<TaskType>,
) -> Result<MessageReplyEnvelope<ReplyType>, String>
{
    let span = span!(
        Level::INFO,
        "Received Task",
        "query_id" = envelope.query_id,
        "task_id" = envelope.task_id,
        "db_id" = ?envelope.db_task_id,
    );
    let _guard = span.enter();

    trace!(
        "Received task. envelope: {:?}",
        envelope
    );
    counter!("zkmr_worker_tasks_received_total").increment(1);
    match std::panic::catch_unwind(|| provers_manager.delegate_proving(&envelope))
    {
        Ok(result) =>
        {
            match result
            {
                Ok(reply) =>
                {
                    trace!(
                        "Sending reply: {:?}",
                        reply
                    );
                    counter!("zkmr_worker_tasks_processed_total").increment(1);
                    Ok(reply)
                },
                Err(e) =>
                {
                    error!(
                        "Error processing task: {:?}",
                        e
                    );
                    counter!("zkmr_worker_error_count", "error_type" =>  "proof processing")
                        .increment(1);

                    Err(format!("{e:?}"))
                },
            }
        },
        Err(panic) =>
        {
            counter!(
                "zkmr_worker_error_count",
                "error_type" => "proof_processing"
            )
            .increment(1);

            let msg = match panic.downcast_ref::<&'static str>()
            {
                Some(s) => *s,
                None =>
                {
                    match panic.downcast_ref::<String>()
                    {
                        Some(s) => &s[..],
                        None => "Box<dyn Any>",
                    }
                },
            };

            error!(
                "panic encountered while proving {} : {msg}",
                envelope.id()
            );
            Err(
                format!(
                    "{}: {msg}",
                    envelope.id()
                ),
            )
        },
    }
}

async fn process_message_from_gateway(
    provers_manager: &mut ProversManager<TaskType, ReplyType>,
    message: &WorkerToGwResponse,
    outbound: &mut tokio::sync::mpsc::Sender<WorkerToGwRequest>,
) -> Result<()>
{
    match &message.response
    {
        Some(response) =>
        {
            match response
            {
                lagrange::worker_to_gw_response::Response::Todo(json_document) =>
                {
                    let message_envelope =
                        serde_json::from_str::<MessageEnvelope<TaskType>>(json_document)?;

                    let reply = tokio::task::block_in_place(
                        move || -> Result<MessageReplyEnvelope<ReplyType>, String> {
                            process_downstream_payload(
                                provers_manager,
                                message_envelope,
                            )
                        },
                    );

                    let outbound_msg = match reply
                    {
                        Ok(reply) =>
                        {
                            WorkerToGwRequest {
                                request: Some(
                                    lagrange::worker_to_gw_request::Request::WorkerDone(
                                        WorkerDone {
                                            reply: Some(
                                                Reply::ReplyString(serde_json::to_string(&reply)?),
                                            ),
                                        },
                                    ),
                                ),
                            }
                        },
                        Err(error_str) =>
                        {
                            WorkerToGwRequest {
                                request: Some(
                                    lagrange::worker_to_gw_request::Request::WorkerDone(
                                        WorkerDone {
                                            reply: Some(Reply::WorkerError(error_str)),
                                        },
                                    ),
                                ),
                            }
                        },
                    };
                    outbound
                        .send(outbound_msg)
                        .await?;

                    counter!("zkmr_worker_grpc_messages_sent_total",
                                    "message_type" => "text")
                    .increment(1);
                },
            }
        },
        None =>
        {
            tracing::warn!("Received WorkerToGwReponse with empty reponse field");
        },
    }
    Ok(())
}

fn get_wallet(config: &Config) -> Result<Wallet<SigningKey>>
{
    let res = match (
        &config
            .avs
            .lagr_keystore,
        &config
            .avs
            .lagr_pwd,
        &config
            .avs
            .lagr_private_key,
    )
    {
        (Some(keystore_path), Some(password), None) =>
        {
            read_keystore(
                keystore_path,
                password.expose_secret(),
            )?
        },
        (Some(_), None, Some(pkey)) =>
        {
            Wallet::from_str(pkey.expose_secret()).context("Failed to create wallet")?
        },
        _ => bail!("Must specify either keystore path w/ password OR private key"),
    };

    Ok(res)
}

fn get_claims(config: &Config) -> Result<Claims>
{
    let registered = RegisteredClaims {
        issuer: Some(
            config
                .avs
                .issuer
                .clone(),
        ),
        subject: Some(
            config
                .avs
                .worker_id
                .clone(),
        ),
        issued_at: Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Epoch can not be in the future")
                .as_secs(),
        ),
        ..Default::default()
    };

    let version = env!("CARGO_PKG_VERSION");
    let private = [
        (
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        ),
        (
            "worker_class".to_string(),
            serde_json::Value::String(
                config
                    .worker
                    .instance_type
                    .to_string(),
            ),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<String, serde_json::Value>>();

    Ok(
        Claims {
            registered,
            private,
        },
    )
}

fn run_with_websocket(config: &Config) -> Result<()>
{
    let lagrange_wallet = get_wallet(config)?;

    info!(
        "Connecting to the Gateway using websocket. gateway_url: {}",
        &config
            .avs
            .gateway_url
    );

    let claims = get_claims(config)?;

    let (mut ws_socket, _) = connect(
        &config
            .avs
            .gateway_url,
    )?;
    counter!("zkmr_worker_gateway_connection_count").increment(1);

    info!("Authenticating");
    let token = JWTAuth::new(
        claims,
        &lagrange_wallet,
    )?
    .encode()?;
    // Send the authentication frame..
    let auth_msg = UpstreamPayload::<ReplyType>::Authentication {
        token,
    };
    let auth_msg_json =
        serde_json::to_string(&auth_msg).context("Failed to serialize Authentication message")?;
    ws_socket
        .send(Message::Text(auth_msg_json))
        .context("Failed to send authorization message")?;
    // ...then wait for the ack.
    ws_socket
        .read()
        .context("Failed to read authentication confirmation")
        .and_then(
            |reply| {
                match reply
                {
                    Message::Text(payload) => Ok(payload),
                    _ =>
                    {
                        bail!(
                "Unexpected websocket message during authentication, expected Text. reply: {}",
                reply
            )
                    },
                }
            },
        )
        .and_then(
            |payload| {
                match serde_json::from_str::<DownstreamPayload<ReplyType>>(&payload)
                {
                    Ok(DownstreamPayload::Ack) => Ok(()),
                    Ok(DownstreamPayload::Todo {
                        envelope,
                    }) =>
                    {
                        bail!(
                            "Unexpected Todo message during authentication. msg: {:?}",
                            envelope
                        )
                    },
                    Err(err) =>
                    {
                        bail!(
                            "Websocket error while authenticating. err: {}",
                            err
                        )
                    },
                }
            },
        )?;

    let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();

    // Always download the checksum files, this is needed by the prover constructor
    let checksum_url = &config
        .public_params
        .checksum_file_url();
    let expected_checksums_file = &config
        .public_params
        .checksum_expected_local_path;
    fetch_checksum_file(
        checksum_url,
        expected_checksums_file,
    )?;

    register_v1_provers(
        config,
        &mut provers_manager,
    )
    .context("Failed to register V1 provers")?;

    if !config
        .public_params
        .skip_checksum
    {
        verify_directory_checksums(
            &config
                .public_params
                .dir,
            expected_checksums_file,
        )
        .context("Public parameters verification failed")?;
    }

    start_work(
        &mut ws_socket,
        &mut provers_manager,
    )?;

    Ok(())
}

fn start_work(
    ws_socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    provers_manager: &mut ProversManager<TaskType, ReplyType>,
) -> Result<()>
{
    let ready = UpstreamPayload::<ReplyType>::Ready;
    let ready_json = serde_json::to_string(&ready).context("Failed to encode Ready message")?;
    ws_socket
        .send(Message::Text(ready_json))
        .context("unable to send ready frame")?;

    loop
    {
        let msg = ws_socket
            .read()
            .context("Failed to read from gateway socket")?;
        match msg
        {
            Message::Text(content) =>
            {
                trace!(
                    "Received message: {:?}",
                    content
                );

                counter!(
                    "zkmr_worker_websocket_messages_received_total",
                    "message_type" => "text",
                )
                .increment(1);

                match serde_json::from_str::<DownstreamPayload<TaskType>>(&content).with_context(
                    || {
                        format!(
                            "Failed to decode msg. content: {}",
                            content
                        )
                    },
                )?
                {
                    DownstreamPayload::Todo {
                        envelope,
                    } =>
                    {
                        let envelope_id = envelope.id();
                        let reply = match process_downstream_payload(
                            provers_manager,
                            envelope,
                        )
                        {
                            Ok(reply) => UpstreamPayload::Done(reply),
                            Err(msg) =>
                            {
                                let var_name = format!(
                                    "{}: {msg}",
                                    envelope_id
                                );
                                UpstreamPayload::ProvingError(var_name)
                            },
                        };
                        counter!("zkmr_worker_websocket_messages_sent_total",
                                    "message_type" => "text")
                        .increment(1);
                        ws_socket.send(Message::Text(serde_json::to_string(&reply)?))?;
                    },
                    DownstreamPayload::Ack =>
                    {
                        counter!(
                            "zkmr_worker_error_count",
                            "error_type" => "unexpected_ack",
                        )
                        .increment(1);
                        bail!("Unexpected ACK frame")
                    },
                }
            },
            Message::Ping(_) =>
            {
                trace!("Received ping or close message");

                counter!(
                    "zkmr_worker_websocket_messages_received_total",
                    "message_type" => "ping",
                )
                .increment(1);
            },
            Message::Close(_) =>
            {
                info!("Received close message");
                return Ok(());
            },
            _ =>
            {
                error!("Unexpected frame: {msg}");
                counter!(
                    "zkmr_worker_error_count",
                    "error_type" => "unexpected_frame",
                )
                .increment(1);
            },
        }
    }
}
