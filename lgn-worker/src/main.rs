use anyhow::*;
use backtrace::Backtrace;
use clap::Parser;
use jwt::{Claims, RegisteredClaims};
use metrics::counter;
use mimalloc::MiMalloc;
use std::fmt::Debug;
use std::net::TcpStream;
use std::result::Result::Ok;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, panic, str::FromStr};
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, info, span, trace, Level};
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
use lgn_messages::types::{DownstreamPayload, ReplyType, TaskType, ToProverType, UpstreamPayload};
use lgn_worker::avs::utils::read_keystore;
use serde::{Deserialize, Serialize};
use tungstenite::stream::MaybeTlsStream;

mod checksum;
mod config;
mod manager;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

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

fn main() {
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

    if let Err(err) = run(cli) {
        error!("Service exiting with an error. err: {:?}", err);
    }
}

fn run(cli: Cli) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    info!("Starting worker. version: {}", version);

    let config = Config::load(cli.config);
    config.validate();
    info!("Loaded configuration. config: {:?}", config);

    let span = span!(
        Level::INFO,
        "Starting node",
        "worker" = config.avs.worker_id.to_string(),
        "issuer" = config.avs.issuer.to_string(),
        "version" = version,
        "class" = config.worker.instance_type.to_string(),
    );
    let _guard = span.enter();

    if let Err(err) = metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.prometheus.port))
        .install()
    {
        bail!("Creating prometheus metrics failed. err: {:?}", err);
    }

    let lagrange_wallet = match (
        &config.avs.lagr_keystore,
        &config.avs.lagr_pwd,
        &config.avs.lagr_private_key,
    ) {
        (Some(keystore_path), Some(password), None) => {
            read_keystore(keystore_path, password.expose_secret())?
        }
        (Some(_), None, Some(pkey)) => {
            Wallet::from_str(pkey.expose_secret()).context("Failed to create wallet")?
        }
        _ => bail!("Must specify either keystore path w/ password OR private key"),
    };

    info!(
        "Connecting to the gateway. url: {}",
        &config.avs.gateway_url,
    );

    let url = url::Url::parse(&config.avs.gateway_url).context("Gateway URL is invalid")?;
    let connection_request = url
        .into_client_request()
        .context("Gateway URL is invalid, not a websocket")?;

    // Perform authentication
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

    let private = [
        (
            "version".to_string(),
            serde_json::Value::String(version.to_string()),
        ),
        (
            "worker_class".to_string(),
            serde_json::Value::String(config.worker.instance_type.to_string()),
        ),
    ]
    .into_iter()
    .collect::<BTreeMap<String, serde_json::Value>>();

    let claims = Claims {
        registered,
        private,
    };

    let (mut ws_socket, _) = connect(connection_request)?;
    counter!("zkmr_worker_gateway_connection_count").increment(1);

    info!("Authenticating");
    let token = JWTAuth::new(claims, &lagrange_wallet)?.encode()?;
    // Send the authentication frame..
    let auth_msg = UpstreamPayload::<ReplyType>::Authentication { token };
    let auth_msg_json =
        serde_json::to_string(&auth_msg).context("Failed to serialize Authentication message")?;
    ws_socket
        .send(Message::Text(auth_msg_json))
        .context("Failed to send authorization message")?;
    // ...then wait for the ack.
    ws_socket
        .read()
        .context("Failed to read authentication confirmation")
        .and_then(|reply| match reply {
            Message::Text(payload) => Ok(payload),
            _ => bail!(
                "Unexpected websocket message during authentication, expected Text. reply: {}",
                reply
            ),
        })
        .and_then(|payload| {
            match serde_json::from_str::<DownstreamPayload<ReplyType>>(&payload) {
                Ok(DownstreamPayload::Ack) => Ok(()),
                Ok(DownstreamPayload::Todo { envelope }) => bail!(
                    "Unexpected Todo message during authentication. msg: {:?}",
                    envelope
                ),
                Err(err) => bail!("Websocket error while authenticating. err: {}", err),
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
    register_v1_provers(&config, &mut provers_manager).context("Failed to register V1 provers")?;

    if !config.public_params.skip_checksum {
        verify_directory_checksums(&config.public_params.dir, expected_checksums_file)
            .context("Public parameters verification failed")?;
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
    info!("Sending ready to work message");
    let ready = UpstreamPayload::<ReplyType>::Ready;
    let ready_json = serde_json::to_string(&ready).context("Failed to encode Ready message")?;
    ws_socket
        .send(Message::Text(ready_json))
        .context("unable to send ready frame")?;

    loop {
        let msg = ws_socket
            .read()
            .context("Failed to read from gateway socket")?;
        match msg {
            Message::Text(content) => {
                trace!("Received message: {:?}", content);

                counter!(
                    "zkmr_worker_websocket_messages_received_total",
                    "message_type" => "text",
                )
                .increment(1);

                match serde_json::from_str::<DownstreamPayload<T>>(&content)
                    .with_context(|| format!("Failed to decode msg. content: {}", content))?
                {
                    DownstreamPayload::Todo { envelope } => {
                        let span = span!(
                            Level::INFO,
                            "Received Todo",
                            "query_id" = envelope.query_id,
                            "task_id" = envelope.task_id,
                            "db_id" = ?envelope.db_task_id,
                        );
                        let _guard = span.enter();
                        debug!("Received Todo. envelope: {:?}", envelope);

                        counter!("zkmr_worker_tasks_received_total").increment(1);
                        match provers_manager.delegate_proving(envelope) {
                            Ok(reply) => {
                                debug!("Sending reply: {:?}", reply);
                                counter!("zkmr_worker_tasks_processed_total").increment(1);

                                let done = UpstreamPayload::Done(reply);
                                let done_json = serde_json::to_string(&done)
                                    .context("Failed to encode Done message")?;
                                ws_socket
                                    .send(Message::Text(done_json))
                                    .context("Failed to send response to gateway socket")?;
                                counter!(
                                    "zkmr_worker_websocket_messages_sent_total",
                                    "message_type" => "text",
                                )
                                .increment(1);
                            }
                            Err(err) => {
                                error!("Error processing task. err: {:?}", err);
                                counter!(
                                    "zkmr_worker_error_count",
                                    "error_type" => "proof_processing",
                                )
                                .increment(1);
                            }
                        }
                    }
                    DownstreamPayload::Ack => {
                        counter!(
                            "zkmr_worker_error_count",
                            "error_type" => "unexpected_ack",
                        )
                        .increment(1);
                        bail!("Unexpected ACK frame")
                    }
                }
            }
            Message::Ping(_) => {
                debug!("Received ping or close message");

                counter!(
                    "zkmr_worker_websocket_messages_received_total",
                    "message_type" => "ping",
                )
                .increment(1);
            }
            Message::Close(_) => {
                info!("Received close message");
                return Ok(());
            }
            _ => {
                error!("Unexpected frame: {msg}");
                counter!(
                    "zkmr_worker_error_count",
                    "error_type" => "unexpected_frame",
                )
                .increment(1);
            }
        }
    }
}
