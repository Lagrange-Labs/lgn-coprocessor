use std::collections::HashMap;

use anyhow::*;
use lgn_messages::types::ProverType;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskDifficulty;
use lgn_messages::types::TaskType;

use crate::config::Config;
use crate::manager::ProversManager;

pub(crate) fn register_v1_provers(
    config: &Config,
    manager: &mut ProversManager<TaskType, ReplyType>,
    checksums: &HashMap<String, blake3::Hash>,
) -> Result<()> {
    if config.worker.instance_type >= TaskDifficulty::Small {
        let query_prover = lgn_provers::provers::v1::query::create_prover(
            &config.public_params.params_base_url(),
            &config.public_params.dir,
            &config.public_params.query_params.file,
            checksums,
        )?;

        manager.add_prover(ProverType::V1Query, Box::new(query_prover));
    }

    if config.worker.instance_type >= TaskDifficulty::Medium {
        let preprocessing_prover = lgn_provers::provers::v1::preprocessing::create_prover(
            &config.public_params.params_base_url(),
            &config.public_params.dir,
            &config.public_params.preprocessing_params.file,
            checksums,
        )?;

        manager.add_prover(ProverType::V1Preprocessing, Box::new(preprocessing_prover));
    }

    if config.worker.instance_type >= TaskDifficulty::Large {
        let groth16_prover = lgn_provers::provers::v1::groth16::create_prover(
            &config.public_params.params_base_url(),
            &config.public_params.dir,
            &config.public_params.groth16_assets.circuit_file,
            checksums,
            &config.public_params.groth16_assets.r1cs_file,
            &config.public_params.groth16_assets.pk_file,
        )
        .context("initializing Groth16 prover")?;

        manager.add_prover(ProverType::V1Groth16, Box::new(groth16_prover));
    }

    Ok(())
}
