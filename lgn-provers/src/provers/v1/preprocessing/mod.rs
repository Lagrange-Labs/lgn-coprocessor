use std::collections::HashMap;

use tracing::info;

use crate::provers::LgnProver;
mod dummy_prover;
pub mod euclid_prover;
pub use euclid_prover::EuclidProver;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<impl LgnProver> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        use dummy_prover::PreprocessingDummyProver;
        info!("Creating dummy preprocessing prover");
        PreprocessingDummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating preprocessing prover");
        euclid_prover::EuclidProver::init(url, dir, file, checksums)?
    };

    info!("Preprocessing prover created");
    Ok(prover)
}
