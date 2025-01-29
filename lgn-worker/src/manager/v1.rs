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
) -> Result<()>
{
    if config
        .worker
        .instance_type
        >= TaskDifficulty::Small
    {
        register_v1_query(
            config,
            manager,
        )
        .context("failed to register the query prover")?;
    }

    if config
        .worker
        .instance_type
        >= TaskDifficulty::Medium
    {
        register_v1_preprocessor(
            config,
            manager,
        )
        .context("failed to register the pre-processing prover")?;
    }

    if config
        .worker
        .instance_type
        >= TaskDifficulty::Large
    {
        register_v1_groth16(
            config,
            manager,
        )
        .context("failed to register the groth16 prover")?;
    }

    Ok(())
}

fn register_v1_preprocessor(
    config: &Config,
    manager: &mut ProversManager<TaskType, ReplyType>,
) -> Result<()>
{
    let params_config = &config.public_params;
    let params_base_url = &params_config.params_base_url();
    let preprocessing_prover = lgn_provers::provers::v1::preprocessing::create_prover(
        params_base_url,
        &params_config.dir,
        &params_config
            .preprocessing_params
            .file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )?;

    manager.add_prover(
        ProverType::V1Preprocessing,
        Box::new(preprocessing_prover),
    );
    Ok(())
}

fn register_v1_query(
    config: &Config,
    manager: &mut ProversManager<TaskType, ReplyType>,
) -> Result<()>
{
    let params_config = &config.public_params;
    let params_base_url = &params_config.params_base_url();
    let query_prover = lgn_provers::provers::v1::query::create_prover(
        params_base_url,
        &params_config.dir,
        &params_config
            .query_params
            .file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        params_config.skip_store,
    )?;

    manager.add_prover(
        ProverType::V1Query,
        Box::new(query_prover),
    );
    Ok(())
}

fn register_v1_groth16(
    config: &Config,
    router: &mut ProversManager<TaskType, ReplyType>,
) -> Result<()>
{
    let params_config = &config.public_params;
    let params_base_url = &params_config.params_base_url();
    let assets = &params_config.groth16_assets;
    let groth16_prover = lgn_provers::provers::v1::groth16::create_prover(
        params_base_url,
        &params_config.dir,
        &assets.circuit_file,
        &params_config.checksum_expected_local_path,
        params_config.skip_checksum,
        &assets.r1cs_file,
        &assets.pk_file,
        params_config.skip_store,
    )?;

    router.add_prover(
        ProverType::V1Groth16,
        Box::new(groth16_prover),
    );
    Ok(())
}
