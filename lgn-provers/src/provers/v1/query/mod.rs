use std::collections::HashMap;

use tracing::debug;
use tracing::info;

use crate::provers::LgnProver;

#[cfg(feature = "dummy-prover")]
pub(crate) mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
pub(crate) mod euclid_prover;
#[cfg(not(feature = "dummy-prover"))]
pub mod task;

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
pub async fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksums: &HashMap<String, blake3::Hash>,
    with_tracing: bool,
) -> anyhow::Result<impl LgnProver> {
    #[cfg(feature = "dummy-prover")]
    let prover = {
        info!("Creating QueryDummyProver");
        dummy_prover::QueryDummyProver
    };

    #[cfg(not(feature = "dummy-prover"))]
    let prover = {
        info!("Creating QueryEuclidProver");
        euclid_prover::QueryEuclidProver::init(url, dir, file, checksums, with_tracing).await?
    };

    debug!("Query prover created");

    Ok(prover)
}
