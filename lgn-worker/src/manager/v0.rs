use crate::config::Config;
use crate::manager::ProversManager;
use lgn_messages::types::{ProverType, ReplyType, TaskType, WorkerClass};
use lgn_provers::provers::v0::{groth16, preprocessing, query};
use tracing::info;

#[allow(dead_code)]
pub(crate) fn register_v0_provers(
    config: &Config,
    router: &mut ProversManager<TaskType, ReplyType>,
) {
    if config.worker.instance_type >= WorkerClass::Small {
        info!("Creating query prover");
        register_v0_ecr721_query_prover(config, router);
        register_v0_ecr20_query_prover(config, router);
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

fn register_v0_groth16_prover(config: &Config, router: &mut ProversManager<TaskType, ReplyType>) {
    let params_config = &config.public_params;
    let assets = &params_config.groth16_assets;
    let groth16_prover = groth16::create_prover(
        &params_config.url,
        &params_config.dir,
        &assets.circuit_file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        &assets.r1cs_file,
        &assets.pk_file,
        params_config.skip_store,
    )
    .expect("Failed to create groth16 handler");

    router.add_prover(ProverType::Query2Groth16, Box::new(groth16_prover));
}

fn register_v0_preprocessor(config: &Config, router: &mut ProversManager<TaskType, ReplyType>) {
    let params_config = &config.public_params;
    let preprocessing_prover = preprocessing::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.preprocessing_params.file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )
    .expect("Failed to create preprocessing handler");

    router.add_prover(ProverType::Query2Preprocess, Box::new(preprocessing_prover));
}

fn register_v0_ecr721_query_prover(
    config: &Config,
    router: &mut ProversManager<TaskType, ReplyType>,
) {
    let params_config = &config.public_params;
    let query2_prover = query::erc721::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.query_params.file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    router.add_prover(ProverType::Query2Query, Box::new(query2_prover));
}

fn register_v0_ecr20_query_prover(
    config: &Config,
    router: &mut ProversManager<TaskType, ReplyType>,
) {
    let params_config = &config.public_params;
    let query3_prover = query::erc20::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.query_params.file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    router.add_prover(ProverType::QueryErc20, Box::new(query3_prover));
}
