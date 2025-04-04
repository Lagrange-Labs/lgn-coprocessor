//! This module contains logic of generating the Groth16 proofs which could be verified on-chain.
use std::collections::HashMap;

use tracing::debug;
use tracing::info;

use crate::provers::LgnProver;

#[cfg(feature = "dummy-prover")]
mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
mod euclid_prover;

#[cfg(not(feature = "dummy-prover"))]
mod task;

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn create_prover(
    url: &str,
    dir: &str,
    circuit_file: &str,
    checksums: &HashMap<String, blake3::Hash>,
    pk_file: &str,
    vk_file: &str,
) -> anyhow::Result<impl LgnProver> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        info!("Creating Groth16DummyProver");
        dummy_prover::Groth16DummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating groth16 prover");
        euclid_prover::Groth16EuclidProver::init(
            url,
            dir,
            circuit_file,
            pk_file,
            vk_file,
            checksums,
        )?
    };

    debug!("Groth16 prover created");
    Ok(prover)
}
