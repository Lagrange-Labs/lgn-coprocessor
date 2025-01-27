use tracing::debug;
use tracing::info;

use crate::provers::v1::preprocessing::prover::StorageDatabaseProver;
use crate::provers::v1::preprocessing::prover::StorageExtractionProver;
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
    checksum_expected_local_path: &str,
    skip_checksum: bool,
    skip_store: bool,
) -> anyhow::Result<Preprocessing<impl StorageExtractionProver + StorageDatabaseProver>> {
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
            euclid_prover::EuclidProver::init(
                url,
                dir,
                file,
                checksum_expected_local_path,
                skip_checksum,
                skip_store,
            )?
        };
        debug!("Preprocessing prover created");
        prover
    };

    Ok(Preprocessing::new(prover))
}
