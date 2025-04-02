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
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<impl LgnProver> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        use dummy_prover::DummyProver;
        info!("Creating dummy preprocessing prover");
        DummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating preprocessing prover");
        euclid_prover::EuclidProver::init(url, dir, file, checksums)?
    };

    info!("Preprocessing prover created");
    Ok(prover)
}
