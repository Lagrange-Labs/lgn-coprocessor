use crate::provers::v1::preprocessing::prover::{StorageDatabaseProver, StorageExtractionProver};
use crate::provers::v1::preprocessing::task::Preprocessing;
use tracing::info;
pub mod euclid_prover;
pub mod prover;
pub mod task;

#[cfg(feature = "dummy-prover")]
mod dummy_prover;

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
        {
            use dummy_prover::DummyProver;
            info!("Creating dummy storage prover");
            DummyProver
        }

        #[cfg(not(feature = "dummy-prover"))]
        {
            info!("Creating storage prover");
            euclid_prover::EuclidProver::init(
                url,
                dir,
                file,
                checksum_expected_local_path,
                skip_checksum,
                skip_store,
            )?
        }
    };

    Ok(Preprocessing::new(prover))
}
