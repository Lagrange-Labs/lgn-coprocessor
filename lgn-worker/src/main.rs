use std::collections::BTreeSet;
use std::path::Path;
use std::result::Result::Ok;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, str::FromStr};
use std::{fs, panic};

use std::fs::File;
use std::io::Write;

use ::metrics::counter;
use anyhow::*;
use backtrace::Backtrace;
use checksums::ops::{
    compare_hashes, create_hashes, read_hashes, write_hash_comparison_results, write_hashes,
    CompareFileResult,
};
use checksums::Error;
use clap::Parser;
use jwt::{Claims, RegisteredClaims};
use mimalloc::MiMalloc;
use tracing::{debug, error, info};
use tracing_subscriber::field::debug;
use tracing_subscriber::EnvFilter;
use tungstenite::client::IntoClientRequest;
use tungstenite::{connect, Message};

use ethers::signers::Wallet;
use lgn_auth::jwt::JWTAuth;
use lgn_messages::types::{DownstreamPayload, ReplyType, TaskType, UpstreamPayload, WorkerClass};
use lgn_provers::provers::v0::{groth16, preprocessing, query};
use lgn_provers::provers::ProverType;
use lgn_worker::avs::utils::read_keystore;

use crate::config::Config;
use crate::manager::ProversManager;
use crate::metrics::Metrics;

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

    let mut provers_manager = ProversManager::new(&metrics);
    register_provers(config, &mut provers_manager);

    // Fetch checksum file
    // generate the checksum with
    // Download the params the install with rust checksums and run checksums -c -r zkmr_params -a BLAKE3
    let checksum_url = &config.public_params.checksum_url;
    let expected_checksums_file = "/tmp/expected_checksums.txt";
    fetch_checksum_file(checksum_url, expected_checksums_file)?;

    // Verify checksum

    verify_checksums(&config.public_params.dir, expected_checksums_file)
        .context("Failed to verify checksums")?;

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
                debug!("Received message: {:?}", content);
                metrics.increment_websocket_messages_received("text");

                match serde_json::from_str::<DownstreamPayload<TaskType>>(&content)? {
                    DownstreamPayload::Todo { envelope } => {
                        debug!("Received task: {:?}", envelope);
                        counter!("zkmr_worker_tasks_received").increment(1);

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

fn register_provers(config: &Config, router: &mut ProversManager) {
    if config.worker.instance_type >= WorkerClass::Small {
        info!("Creating query prover");
        register_v0_query_prover(config, router);
        info!("Query prover created");
    }

    if config.worker.instance_type >= WorkerClass::Medium {
        info!("Creating preprocessing prover");
        register_v0_preprocessor(config, router);
        info!("Preprocessing prover created");
    }

    if config.worker.instance_type >= WorkerClass::Large {
        info!("Creating groth16 prover");
        register_v0_groth16_prover(config, router);
        info!("Groth16 prover created");
    }
}

fn register_v0_groth16_prover(config: &Config, router: &mut ProversManager) {
    let params_config = &config.public_params;
    let assets = &params_config.groth16_assets;
    let groth16_prover = groth16::create_prover(
        &params_config.url,
        &params_config.dir,
        &assets.circuit_file,
        &assets.r1cs_file,
        &assets.pk_file,
        params_config.skip_store,
    )
    .expect("Failed to create groth16 handler");

    router.add_prover(ProverType::Query2Groth16, Box::new(groth16_prover));
}

fn register_v0_preprocessor(config: &Config, router: &mut ProversManager) {
    let params_config = &config.public_params;
    let preprocessing_prover = preprocessing::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.preprocessing_params.file,
        params_config.skip_store,
    )
    .expect("Failed to create preprocessing handler");

    router.add_prover(ProverType::Query2Preprocess, Box::new(preprocessing_prover));
}

fn register_v0_query_prover(config: &Config, router: &mut ProversManager) {
    let params_config = &config.public_params;
    let query2_prover = query::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.query2_params.file,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    router.add_prover(ProverType::Query2Query, Box::new(query2_prover));
}

fn verify_checksums(dir: &str, expected_checksums_file: &str) -> anyhow::Result<()> {
    debug!("Computing hashes from: {:?}", dir);
    let computed_hashes = create_hashes(
        Path::new(dir),
        BTreeSet::new(),
        checksums::Algorithm::BLAKE3,
        None,
        true,
        3,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    debug!("Computed hashes: {:?}", computed_hashes);
    write_hashes(
        &(
            "output".to_string(),
            Path::new("public_params.hash").to_path_buf(),
        ),
        checksums::Algorithm::BLAKE3,
        computed_hashes.clone(),
    );
    let expected_hashes_file = Path::new(&expected_checksums_file);
    let expected_hashes = read_hashes(
        &mut std::io::stderr(),
        &("output".to_string(), expected_hashes_file.to_path_buf()),
    );
    debug!(
        "expected hashes from: {:?} is {:?}",
        expected_hashes_file, expected_hashes
    );
    let compare_hashes =
        compare_hashes("compare_hashes", computed_hashes, expected_hashes.unwrap());
    debug!("compare hashes: {:?} ", compare_hashes);

    let result = write_hash_comparison_results(
        &mut std::io::stdout(),
        &mut std::io::stderr(),
        compare_hashes.clone(),
    );
    debug!("checksum result: {:?} ", result);

    match result {
        Error::NoError => {
            // Test result no error
            info!("Checksum is successful");
        }
        Error::NFilesDiffer(count) => {
            if let Ok((_, file_results)) = &compare_hashes {
                let file_differs: Vec<&CompareFileResult> = file_results
                    .iter()
                    .filter(|f| {
                        if let CompareFileResult::FileDiffers { .. } = f {
                            true
                        } else {
                            false
                        }
                    })
                    .collect();

                for file_differ in file_differs {
                    if let CompareFileResult::FileDiffers { file, .. } = file_differ {
                        info!("File did not match the checksum. Deleting File {} ", file);
                        // This will only delete the file where the checksum has failed
                        //if let Err(err) = fs::remove_file(Path::new(dir).join(file)) {
                        //    error!("Error deleting file {}: {}", file, err);
                        //}
                        // Temporarily delete the whole pp dir, because the download part doesnt handle yet downloading only the missing files
                        if let Err(err) = fs::remove_dir_all(Path::new(dir)) {
                            error!("Error deleting dir {}: {}", dir, err);
                        }
                    }
                }
            } else {
                error!("Failed to get file comparison results");
            }
            bail!("{} files do not match", count);
        }
        _ => {
            error!("Checksum failure: {:?}", result)
        }
    }

    Ok(())
}
fn fetch_checksum_file(url: &str, local_path: &str) -> anyhow::Result<()> {
    let response = reqwest::blocking::get(url)
        .context("Failed to fetch checksum file")?
        .text()
        .context("Failed to read response text")?;

    let mut file = File::create(local_path).context("Failed to create local checksum file")?;
    file.write_all(response.as_bytes())
        .context("Failed to write checksum file")?;

    Ok(())
}
