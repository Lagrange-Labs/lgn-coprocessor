use std::collections::HashMap;

use tracing::debug;
use tracing::info;

use crate::provers::v1::preprocessing::prover::PreprocessingProver;
use crate::provers::v1::preprocessing::task::Preprocessing;
pub mod prover;
pub mod task;

#[cfg(feature = "dummy-prover")]
mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
pub mod euclid_prover;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<Preprocessing<impl PreprocessingProver>> {
    let prover = {
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
        debug!("Preprocessing prover created");
        prover
    };

    Ok(Preprocessing::new(prover))
}
