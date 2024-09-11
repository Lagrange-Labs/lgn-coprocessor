use crate::config::Config;
use crate::manager::ProversManager;
use lgn_messages::types::{ProverType, ReplyType, TaskDifficulty, TaskType};
use tracing::info;

pub(crate) fn register_v1_provers(
    config: &Config,
    manager: &mut ProversManager<TaskType, ReplyType>,
) {
    if config.worker.instance_type >= TaskDifficulty::Small {
        info!("Creating v1 query prover");
        register_v1_query(config, manager);
        info!("Query prover created");
    }

    if config.worker.instance_type >= TaskDifficulty::Medium {
        info!("Creating v1 preprocessing prover");
        register_v1_preprocessor(config, manager);
        info!("Preprocessing prover created");
    }

    if config.worker.instance_type >= TaskDifficulty::Large {
        info!("Creating groth16 prover");
        register_v1_groth16(config, manager);
        info!("Groth16 prover created");
    }
}

fn register_v1_preprocessor(config: &Config, manager: &mut ProversManager<TaskType, ReplyType>) {
    let params_config = &config.public_params;
    let preprocessing_prover = lgn_provers::provers::v1::preprocessing::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.preprocessing_params.file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )
    .expect("Failed to create preprocessing handler");

    manager.add_prover(ProverType::V1Preprocessing, Box::new(preprocessing_prover));
}

fn register_v1_query(config: &Config, manager: &mut ProversManager<TaskType, ReplyType>) {
    let params_config = &config.public_params;
    let query_prover = lgn_provers::provers::v1::query::create_prover(
        &params_config.url,
        &params_config.dir,
        &params_config.query_params.file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )
    .expect("Failed to create query handler");

    manager.add_prover(ProverType::V1Query, Box::new(query_prover));
}

fn register_v1_groth16(config: &Config, router: &mut ProversManager<TaskType, ReplyType>) {
    let params_config = &config.public_params;
    let assets = &params_config.groth16_assets;
    let groth16_prover = lgn_provers::provers::v1::groth16::create_prover(
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

    router.add_prover(ProverType::V1Groth16, Box::new(groth16_prover));
}
