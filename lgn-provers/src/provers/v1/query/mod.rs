use std::collections::HashMap;

use tracing::info;

pub(crate) mod dummy_prover;
pub(crate) mod euclid_prover;
pub use euclid_prover::EuclidQueryProver;

use super::V1Prover;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
) -> anyhow::Result<impl V1Prover> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        use dummy_prover::QueryDummyProver;
        info!("Creating dummy query prover");
        QueryDummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating query prover");

        euclid_prover::EuclidQueryProver::init(url, dir, file, checksums)?
    };

    info!("Query prover created");

    Ok(prover)
}
