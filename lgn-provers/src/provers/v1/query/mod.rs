use crate::provers::v1::query::prover::StorageQueryProver;
use crate::provers::v1::query::task::Querying;
use tracing::info;

pub(crate) mod dummy_prover;
pub(crate) mod euclid_prover;
pub(crate) mod prover;
pub mod task;

pub const MAX_NUM_RESULT_OPS: usize = 20;
pub const MAX_NUM_RESULTS: usize = 10;
pub const MAX_NUM_OUTPUTS: usize = 3;
pub const MAX_NUM_ITEMS_PER_OUTPUT: usize = 5;
pub const MAX_NUM_PLACEHOLDERS: usize = 10;
pub const MAX_NUM_COLUMNS: usize = 20;
pub const MAX_NUM_PREDICATE_OPS: usize = 20;
#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksum_expected_local_path: &str,
    skip_checksum: bool,
    skip_store: bool,
) -> anyhow::Result<Querying<impl StorageQueryProver>> {
    let prover = {
        #[cfg(feature = "dummy-prover")]
        {
            use dummy_prover::DummyProver;
            info!("Creating dummy storage prover");
            DummyProver
        }

        #[cfg(not(feature = "dummy-prover"))]
        {
            info!("Creating storage prover");

            euclid_prover::EuclidQueryProver::init(
                url,
                dir,
                file,
                checksum_expected_local_path,
                skip_checksum,
                skip_store,
            )?
        }
    };

    Ok(Querying::new(prover))
}
