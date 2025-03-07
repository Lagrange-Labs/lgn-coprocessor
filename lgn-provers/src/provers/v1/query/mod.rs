use std::collections::HashMap;

use tracing::info;

use crate::provers::LgnProver;

pub(crate) mod dummy_prover;
pub(crate) mod euclid_prover;
pub use euclid_prover::EuclidQueryProver;

pub const ROW_TREE_MAX_DEPTH: usize = 25;
pub const INDEX_TREE_MAX_DEPTH: usize = 26;
pub const MAX_NUM_RESULT_OPS: usize = 20;
pub const MAX_NUM_RESULTS: usize = 10;
pub const MAX_NUM_OUTPUTS: usize = 5;
pub const MAX_NUM_ITEMS_PER_OUTPUT: usize = 5;
pub const MAX_NUM_PLACEHOLDERS: usize = 5;
pub const MAX_NUM_COLUMNS: usize = 20;
pub const MAX_NUM_PREDICATE_OPS: usize = 20;

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
        info!("Creating dummy query prover");
        DummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating query prover");

        euclid_prover::EuclidQueryProver::init(url, dir, file, checksums)?
    };

    info!("Query prover created");

    Ok(prover)
}
