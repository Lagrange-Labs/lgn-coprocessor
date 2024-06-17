use self::prover::QueryProver;
use crate::provers::v0::query::task::Query;
use tracing::info;

mod dummy_prover;
pub mod prover;
mod task;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksum_expected_local_path: &str,
    skip_store: bool,
) -> anyhow::Result<Query<impl QueryProver>> {
    let prover = {
        #[cfg(feature = "dummy-prover")]
        {
            info!("Creating dummy query prover");
            dummy_prover::DummyProver
        }

        #[cfg(not(feature = "dummy-prover"))]
        {
            use crate::provers::v0::query::prover::QueryStorageProver;

            info!("Creating query prover");
            QueryStorageProver::init(url, dir, file, checksum_expected_local_path, skip_store)?
        }
    };

    Ok(Query::new(prover))
}
