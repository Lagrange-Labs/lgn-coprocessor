use anyhow::*;
use backtrace::Backtrace;
use clap::Parser;
use jwt::{Claims, RegisteredClaims};
use mimalloc::MiMalloc;
use std::fmt::Debug;
use std::net::TcpStream;
use std::result::Result::Ok;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, panic, str::FromStr};
use tracing::{debug, error, info, trace};
use tracing_subscriber::EnvFilter;
use tungstenite::client::IntoClientRequest;
use tungstenite::{connect, Message, WebSocket};

use crate::checksum::{fetch_checksum_file, verify_directory_checksums};
use crate::config::Config;
use crate::manager::v1::register_v1_provers;
use crate::manager::ProversManager;
use crate::metrics::Metrics;
use ethers::signers::Wallet;
use lgn_auth::jwt::JWTAuth;
use lgn_messages::types::{DownstreamPayload, ReplyType, TaskType, ToProverType, UpstreamPayload};
use lgn_worker::avs::utils::read_keystore;
use serde::{Deserialize, Serialize};
use tungstenite::stream::MaybeTlsStream;

mod checksum;
mod config;
mod manager;
mod metrics;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser, Clone, Debug)]
struct Cli {
    #[clap(short, long)]
    config: Option<String>,

    #[clap(short, long, action)]
    json: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    } else {
        tracing_subscriber::fmt()
            .pretty()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    let config = Config::load(cli.config);
    config.validate();
    info!("Loaded configuration: {:?}", config);

    panic::set_hook(Box::new(|panic_info| {
        let backtrace = Backtrace::new();
        error!("Panic occurred: {:?}", panic_info);
        error!("Backtrace: {:?}", backtrace);
    }));

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], config.prometheus.port))
        .install()?;

    run(&config)
}

fn run(config: &Config) -> Result<()> {
    let metrics = Metrics::new();
    let lagrange_wallet = match (
        &config.avs.lagr_keystore,
        &config.avs.lagr_pwd,
        &config.avs.lagr_private_key,
    ) {
        (Some(keystore_path), Some(password), None) => {
            read_keystore(keystore_path, password.expose_secret())?
        }
        (Some(_), None, Some(pkey)) => Wallet::from_str(pkey.expose_secret())?,
        _ => bail!("Must specify either keystore path w/ password OR private key"),
    };

    // Connect to the WS server
    info!("Connecting to the gateway at {}", &config.avs.gateway_url);

    // Prepare the connection request
    let url = url::Url::parse(&config.avs.gateway_url).with_context(|| "while parsing url")?;
    let connection_request = url
        .into_client_request()
        .with_context(|| "while creating connection request")?;

    // Perform authentication
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

    let claims = Claims {
        registered,
        private,
    };

    // Connect to the server
    let (mut ws_socket, _) = connect(connection_request)?;
    metrics.increment_gateway_connection_count();
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

    if !config.public_params.skip_checksum {
        // Fetch checksum file
        // The checksum file can be generated in two ways.
        // 1- Run the worker, and it will download and spit out the checksum on disk
        // 2- Manually download the params then install with the checksums bin crate and run checksums -c -r zkmr_params -a BLAKE3
        let checksum_url = &config.public_params.checksum_url;
        let expected_checksums_file = &config.public_params.checksum_expected_local_path;
        fetch_checksum_file(checksum_url, expected_checksums_file)?;
    }

    let mut provers_manager = ProversManager::<TaskType, ReplyType>::new(&metrics);
    register_v1_provers(config, &mut provers_manager);

    if !config.public_params.skip_checksum {
        verify_directory_checksums(&config.public_params.dir, expected_checksums_file)
            .context("Failed to verify checksums")?;
    }

    start_work(&metrics, &mut ws_socket, &mut provers_manager)?;

    Ok(())
}

fn start_work<T, R>(
    metrics: &Metrics,
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
                metrics.increment_websocket_messages_received("text");

                match serde_json::from_str::<DownstreamPayload<T>>(&content)? {
                    DownstreamPayload::Todo { envelope } => {
                        debug!("Received task: {:?}", envelope);
                        match provers_manager.delegate_proving(envelope) {
                            Ok(reply) => {
                                debug!("Sending reply: {:?}", reply);

                                ws_socket.send(Message::Text(serde_json::to_string(
                                    &UpstreamPayload::Done(reply),
                                )?))?;
                                metrics.increment_websocket_messages_sent("text");
                            }
                            Err(e) => {
                                error!("Error processing task: {:?}", e);
                                metrics.increment_error_count("proof processing");
                            }
                        }
                    }
                    DownstreamPayload::Ack => bail!("unexpected ACK frame"),
                }
            }
            Message::Ping(_) => {
                debug!("Received ping or close message");
                metrics.increment_websocket_messages_received("ping");
            }
            Message::Close(_) => {
                info!("Received close message");
                return Ok(());
            }
            _ => {
                error!("unexpected frame: {msg}");
                metrics.increment_error_count("unexpected frame");
            }
        }
    }
}
