use anyhow::*;
use backtrace::Backtrace;
use clap::Parser;
use jwt::{Claims, RegisteredClaims};
use k256::ecdsa::SigningKey;
use lagrange::worker_done::Reply;
use lagrange::{WorkerDone, WorkerToGwRequest, WorkerToGwResponse};
use metrics::counter;
use mimalloc::MiMalloc;
use std::fmt::Debug;
use std::io::Write;
use std::net::TcpStream;
use std::result::Result::Ok;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, panic, str::FromStr};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, info, trace};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;
use tungstenite::client::IntoClientRequest;
use tungstenite::{connect, Message, WebSocket};

use crate::checksum::{fetch_checksum_file, verify_directory_checksums};
use crate::config::Config;
use crate::manager::v1::register_v1_provers;
use crate::manager::ProversManager;
use ethers::signers::Wallet;
use lgn_auth::jwt::JWTAuth;
use lgn_messages::types::{
    DownstreamPayload, MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, ToProverType,
    UpstreamPayload,
};
use lgn_worker::avs::utils::read_keystore;
use serde::{Deserialize, Serialize};
use tungstenite::stream::MaybeTlsStream;

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
    #[clap(short, long)]
    config: Option<String>,

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

    panic::set_hook(Box::new(|panic_info| {
        let msg = match panic_info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match panic_info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
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
    info!("Loaded configuration: {:?}", config);

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.prometheus.port))
        .install()?;

    if let Some(grpc_url) = &config.avs.gateway_grpc_url {
        run_with_grpc(&config, grpc_url).await?;
    } else {
        tokio::task::block_in_place(move || -> Result<()> {
            run(&config)?;
            Ok(())
        })?;
    }

    Ok(())
}

async fn maybe_download_checksum(config: &Config) -> Result<()> {
    if config.public_params.skip_checksum {
        return Ok(());
    }

    // Fetch checksum file
    // The checksum file can be generated in two ways.
    // 1- Run the worker, and it will download and spit out the checksum on disk
    // 2- Manually download the params then install with the checksums bin crate and run checksums -c -r zkmr_params -a BLAKE3
    let checksum_url = &config.public_params.checksum_url;
    let expected_checksums_file = &config.public_params.checksum_expected_local_path;

    let response = reqwest::get(checksum_url)
        .await
        .context("Failed to fetch checksum file")?
        .text()
        .await
        .context("Failed to read response text")?;

    let mut file = tokio::fs::File::create(expected_checksums_file)
        .await
        .context("Failed to create local checksum file")?;

    file.write_all(response.as_bytes())
        .await
        .context("Failed to write checksum file")?;

    drop(file);

    Ok(())
}

async fn run_with_grpc(config: &Config, grpc_url: &str) -> Result<()> {
    let uri = grpc_url.parse::<tonic::transport::Uri>()?;
    let (mut outbound, outbound_rx) = tokio::sync::mpsc::channel(1024);

    info!("Verifying the checksums");

    maybe_download_checksum(config).await?;

    let mut provers_manager =
        tokio::task::block_in_place(move || -> Result<ProversManager<TaskType, ReplyType>> {
            let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
            register_v1_provers(config, &mut provers_manager)
                .context("while registering provers")?;
            Ok(provers_manager)
        })?;

    if !config.public_params.skip_checksum {
        verify_directory_checksums(
            &config.public_params.dir,
            &config.public_params.checksum_expected_local_path,
        )
        .context("Failed to verify checksums")?;
    }

    info!("Connecting to GW at uri {uri}");
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
                        error!("connection to the gateway ended with a status: {e}");
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

fn process_downstream_payload<T, R>(
    provers_manager: &mut ProversManager<T, R>,
    envelope: MessageEnvelope<T>,
) -> Result<Option<MessageReplyEnvelope<R>>>
where
    T: ToProverType + for<'a> Deserialize<'a> + Debug + Clone,
    R: Serialize + Debug + Clone,
{
    debug!("Received task: {:?}", envelope);
    counter!("zkmr_worker_tasks_received_total").increment(1);
    match provers_manager.delegate_proving(envelope) {
        Ok(reply) => {
            debug!("Sending reply: {:?}", reply);
            counter!("zkmr_worker_tasks_processed_total").increment(1);
            return Ok(Some(reply));
        }
        Err(e) => {
            error!("Error processing task: {:?}", e);
            counter!("zkmr_worker_error_count", "error_type" =>  "proof processing").increment(1);
        }
    }

    Ok(None)
}

async fn process_message_from_gateway(
    provers_manager: &mut ProversManager<TaskType, ReplyType>,
    message: &WorkerToGwResponse,
    outbound: &mut tokio::sync::mpsc::Sender<WorkerToGwRequest>,
) -> Result<()> {
    match &message.response {
        Some(response) => match response {
            lagrange::worker_to_gw_response::Response::Todo(json_document) => {
                let message_envelope =
                    serde_json::from_str::<MessageEnvelope<TaskType>>(json_document)?;

                let reply = tokio::task::block_in_place(
                    move || -> Result<Option<MessageReplyEnvelope<ReplyType>>> {
                        process_downstream_payload(provers_manager, message_envelope)
                    },
                )?;

                if let Some(reply) = reply {
                    let request = WorkerToGwRequest {
                        request: Some(lagrange::worker_to_gw_request::Request::WorkerDone(
                            WorkerDone {
                                reply: Some(Reply::ReplyString(serde_json::to_string(&reply)?)),
                            },
                        )),
                    };
                    outbound.send(request).await?;
                }
            }
        },
        None => {
            tracing::warn!("Received WorkerToGwReponse with empty reponse field");
        }
    }
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
        }
        (Some(_), None, Some(pkey)) => Wallet::from_str(pkey.expose_secret()).context(format!(
            "while parsing private key {}",
            pkey.expose_secret()
        ))?,
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
                .unwrap()
                .as_secs(),
        ),
        ..Default::default()
    };

    let private = [
        (
            "version".to_string(),
            serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string()),
        ),
        (
            "worker_class".to_string(),
            serde_json::Value::String(config.worker.instance_type.to_string()),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<String, serde_json::Value>>();

    Ok(Claims {
        registered,
        private,
    })
}

fn run(config: &Config) -> Result<()> {
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    let lagrange_wallet = get_wallet(config)?;

    // Connect to the WS server
    info!("Connecting to the gateway at {}", &config.avs.gateway_url);

    // Prepare the connection request
    let url = url::Url::parse(&config.avs.gateway_url).with_context(|| "while parsing url")?;
    let connection_request = url
        .into_client_request()
        .with_context(|| "while creating connection request")?;

    // Perform authentication
    let claims = get_claims(config)?;

    // Connect to the server
    let (mut ws_socket, _) = connect(connection_request)?;
    counter!("zkmr_worker_gateway_connection_count").increment(1);
    info!("Connected to the gateway");

    info!("Authenticating");
    // Sign the JWT claims and encode to (Base64) bytes.
    let token = JWTAuth::new(claims, &lagrange_wallet)?.encode()?;
    // Send the authentication frame..
    ws_socket
        .send(Message::Text(
            serde_json::to_string(&UpstreamPayload::<ReplyType>::Authentication { token }).unwrap(),
        ))
        .context("unable to send authorization frame")?;
    // ...then wait for the ack.
    ws_socket
        .read()
        .context("while waiting for authentication confirmation")
        .and_then(|reply| match reply {
            Message::Text(payload) => Ok(payload),
            _ => bail!("expected ACK frame, found `{reply}`"),
        })
        .and_then(|payload| {
            if let DownstreamPayload::Ack =
                serde_json::from_str::<DownstreamPayload<ReplyType>>(&payload).unwrap()
            {
                info!("connection successful");
                Ok(())
            } else {
                bail!("authentication issue: expected ACK frame, found `{payload}`")
            }
        })?;

    let mut expected_checksums_file = &String::new();

    if !config.public_params.skip_checksum {
        // Fetch checksum file
        // The checksum file can be generated in two ways.
        // 1- Run the worker, and it will download and spit out the checksum on disk
        // 2- Manually download the params then install with the checksums bin crate and run checksums -c -r zkmr_params -a BLAKE3
        let checksum_url = &config.public_params.checksum_url;
        expected_checksums_file = &config.public_params.checksum_expected_local_path;
        fetch_checksum_file(checksum_url, expected_checksums_file)?;
    }

    let mut provers_manager = ProversManager::<TaskType, ReplyType>::new();
    register_v1_provers(config, &mut provers_manager).context("while registering provers")?;

    if !config.public_params.skip_checksum {
        verify_directory_checksums(&config.public_params.dir, expected_checksums_file)
            .context("Failed to verify checksums")?;
    }

    start_work(&mut ws_socket, &mut provers_manager)?;

    Ok(())
}

fn start_work<T, R>(
    ws_socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    provers_manager: &mut ProversManager<T, R>,
) -> Result<()>
where
    T: ToProverType + for<'a> Deserialize<'a> + Debug + Clone,
    R: Serialize + Debug + Clone,
{
    info!("ready to work");
    ws_socket
        .send(Message::Text(
            serde_json::to_string(&UpstreamPayload::<ReplyType>::Ready).unwrap(),
        ))
        .context("unable to send ready frame")?;

    loop {
        let msg = ws_socket
            .read()
            .with_context(|| "unable to read from gateway socket")?;
        match msg {
            Message::Text(content) => {
                trace!("Received message: {:?}", content);

                counter!("zkmr_worker_websocket_messages_received_total", "message_type" => "text")
                    .increment(1);

                match serde_json::from_str::<DownstreamPayload<T>>(&content)? {
                    DownstreamPayload::Todo { envelope } => {
                        debug!("Received task: {:?}", envelope);
                        counter!("zkmr_worker_tasks_received_total").increment(1);
                        let reply = match provers_manager.delegate_proving(envelope.clone()) {
                            Ok(reply) => {
                                debug!("Sending reply: {:?}", reply);
                                counter!("zkmr_worker_tasks_processed_total").increment(1);

                                counter!("zkmr_worker_websocket_messages_sent_total",
                                    "message_type" => "text")
                                .increment(1);
                                UpstreamPayload::Done(reply)
                            }
                            Err(e) => {
                                let filename = format!("{}.json", envelope.task_id);
                                error!(
                                    "error processing task; attempting to save envelope in `{}`: {:?}",
                                    filename, e
                                );
                                if let Err(e) = std::fs::File::create(&filename)
                                    .map(|mut f| f.write_all(content.as_bytes()))
                                {
                                    error!("failed to store failing inputs: {e:?}")
                                }

                                counter!("zkmr_worker_error_count",
                                    "error_type" => "proof
                                    processing")
                                .increment(1);
                                UpstreamPayload::ProvingError(format!("{e:?}"))
                            }
                        };
                        ws_socket.send(Message::Text(serde_json::to_string(&reply)?))?;
                    }
                    DownstreamPayload::Ack => bail!("unexpected ACK frame"),
                }
            }
            Message::Ping(_) => {
                debug!("Received ping or close message");
                counter!("zkmr_worker_websocket_messages_received_total", "message_type" => "ping")
                    .increment(1);
            }
            Message::Close(_) => {
                info!("Received close message");
                return Ok(());
            }
            _ => {
                error!("unexpected frame: {msg}");
                counter!("zkmr_worker_error_count", "error_type" => "unexpected frame")
                    .increment(1);
            }
        }
    }
}
