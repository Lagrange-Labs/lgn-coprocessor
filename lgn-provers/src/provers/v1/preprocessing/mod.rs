use std::collections::HashMap;

use tracing::info;

use crate::provers::LgnProver;

#[cfg(feature = "dummy-prover")]
mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
pub mod euclid_prover;

#[cfg(not(feature = "dummy-prover"))]
pub mod task;

#[allow(unused_variables)]
pub async fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<impl LgnProver> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        info!("Creating PreprocessingDummyProver");
        dummy_prover::PreprocessingDummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating PreprocessingEuclidProver");
        euclid_prover::PreprocessingEuclidProver::init(url, dir, file, checksums).await?
    };

    info!("Preprocessing prover created");
    Ok(prover)
}
