use std::panic;
use std::result::Result::Ok;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, str::FromStr};

use anyhow::*;
use backtrace::Backtrace;
use clap::Parser;

use ::metrics::counter;
use jwt::{Claims, RegisteredClaims};
use mimalloc::MiMalloc;
use tracing::{debug, error, info, trace};
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
        register_v0_ecr721_query_prover(config, router);
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

fn register_v0_ecr721_query_prover(config: &Config, router: &mut ProversManager) {
    let params_config = &config.public_params;
    let query2_prover = query::erc721::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.query2_params.file,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    router.add_prover(ProverType::Query2Query, Box::new(query2_prover));
}

fn register_v0_ecr20_query_prover(config: &Config, router: &mut ProversManager) {
    let params_config = &config.public_params;
    let query3_prover = query::erc20::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.erc20_params.file,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    router.add_prover(ProverType::QueryErc20, Box::new(query3_prover));
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use backtrace::Backtrace;
    use ethers::abi::{ethereum_types, Address};
    use ethers::types::U256;
    use lgn_messages::routing::RoutingKey;
    use lgn_messages::types::v0::query::erc20::{
        BlockPartialNodeInput, BlocksDbData, StateInput, StorageBranchInput, StorageData,
        StorageLeafInput, WorkerTask, WorkerTaskType,
    };
    use lgn_messages::types::{
        HashOutput, MessageEnvelope, Position, ReplyType, TaskType, WorkerReply,
    };
    use lgn_provers::provers::v0::query;
    use lgn_provers::provers::LgnProver;
    use rand::{thread_rng, Rng};
    use std::panic;
    use tracing::error;
    use tracing_subscriber::EnvFilter;

    #[test]
    fn test_ecr20_query() {
        let mut rng = thread_rng();
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();

        panic::set_hook(Box::new(|panic_info| {
            let backtrace = Backtrace::new();
            error!("Panic occurred: {:?}", panic_info);
            error!("Backtrace: {:?}", backtrace);
        }));

        let config = Config::load(Some(
            "/Users/andrussalumets/IdeaProjects/lgn-coprocessor/local_assets/worker-conf.toml"
                .to_string(),
        ));
        let params_config = &config.public_params;
        let mut query3_prover = query::erc20::create_prover(
            &params_config.url,
            &params_config.dir,
            &params_config.erc20_params.file,
            params_config.skip_store,
        )
        .expect("Failed to create query handler");

        let contract = Address::random();
        let address = contract;

        let max_total_supply = U256::MAX >> 16;
        let [value, total_supply] = [0; 2].map(|_| U256(rng.gen::<[u64; 4]>()));
        let total_supply = total_supply & max_total_supply;

        let value = value & total_supply;
        let rewards_rate = U256::from(rng.gen::<u16>());

        let storage_leaf_task = WorkerTask {
            chain_id: 10,
            contract,
            task_type: WorkerTaskType::StorageEntry(StorageData::StorageLeaf(StorageLeafInput {
                block_number: 100,
                position: Position::default(),
                query_address: address,
                value,
                total_supply,
                rewards_rate,
            })),
        };

        let task_type = TaskType::Erc20Query(storage_leaf_task);

        let message = MessageEnvelope::new(
            "query_id".to_string(),
            "task_id".to_string(),
            task_type,
            RoutingKey::Priority(0),
        );

        let result = query3_prover.run(message).unwrap();
        let proof =
            if let ReplyType::Erc20Query(WorkerReply { proof, .. }) = result.inner().unwrap() {
                proof.clone().unwrap()
            } else {
                panic!("Unexpected reply type");
            };

        let storage_leaf_task = WorkerTask {
            chain_id: 10,
            contract,
            task_type: WorkerTaskType::StorageEntry(StorageData::StorageBranch(
                StorageBranchInput {
                    block_number: 100,
                    position: Position::default(),
                    left_child: proof.1,
                    right_child: HashOutput::default().to_vec(),
                    proved_is_right: false,
                },
            )),
        };

        let task_type = TaskType::Erc20Query(storage_leaf_task);

        let message = MessageEnvelope::new(
            "query_id".to_string(),
            "task_id".to_string(),
            task_type,
            RoutingKey::Priority(0),
        );

        let result = query3_prover.run(message).unwrap();

        let proof =
            if let ReplyType::Erc20Query(WorkerReply { proof, .. }) = result.inner().unwrap() {
                proof.clone().unwrap()
            } else {
                panic!("Unexpected reply type");
            };

        let storage_leaf_task = WorkerTask {
            chain_id: 10,
            contract,
            task_type: WorkerTaskType::StateEntry(StateInput {
                smart_contract_address: Default::default(),
                mapping_slot: 8,
                length_slot: 2,
                block_number: 100,
                proof: None,
                block_hash: HashOutput::default(),
                storage_proof: proof.1,
            }),
        };

        let task_type = TaskType::Erc20Query(storage_leaf_task);

        let message = MessageEnvelope::new(
            "query_id".to_string(),
            "task_id".to_string(),
            task_type,
            RoutingKey::Priority(0),
        );

        let result = query3_prover.run(message).unwrap();

        let proof =
            if let ReplyType::Erc20Query(WorkerReply { proof, .. }) = result.inner().unwrap() {
                proof.clone().unwrap()
            } else {
                panic!("Unexpected reply type");
            };

        let storage_leaf_task = WorkerTask {
            chain_id: 10,
            contract,
            task_type: WorkerTaskType::BlocksDb(BlocksDbData::BlockPartialNode(
                BlockPartialNodeInput {
                    position: Default::default(),
                    child_proof: proof.1,
                    sibling_hash: HashOutput::default(),
                    sibling_is_left: false,
                },
            )),
        };

        let task_type = TaskType::Erc20Query(storage_leaf_task);

        let message = MessageEnvelope::new(
            "query_id".to_string(),
            "task_id".to_string(),
            task_type,
            RoutingKey::Priority(0),
        );

        let result = query3_prover.run(message).unwrap();

        let proof =
            if let ReplyType::Erc20Query(WorkerReply { proof, .. }) = result.inner().unwrap() {
                proof.clone().unwrap()
            } else {
                panic!("Unexpected reply type");
            };
    }
}
