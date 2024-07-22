use crate::config::Config;
use crate::manager::ProversManager;
use lgn_messages::types::{ProverType, ReplyType, TaskType, WorkerClass};
use tracing::info;

pub(crate) fn register_v1_provers(
    config: &Config,
    manager: &mut ProversManager<TaskType, ReplyType>,
) {
    if config.worker.instance_type >= WorkerClass::Medium {
        info!("Creating v1 preprocessing prover");
        register_v1_preprocessor(config, manager);
        info!("Preprocessing prover created");
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
