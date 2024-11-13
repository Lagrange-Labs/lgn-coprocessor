use tracing::info;

use crate::provers::v1::query::prover::StorageQueryProver;
use crate::provers::v1::query::task::Querying;

pub(crate) mod prover;
pub mod task;

#[cfg(feature = "dummy-prover")]
pub(crate) mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
pub(crate) mod euclid_prover;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksum_expected_local_path: &str,
    skip_checksum: bool,
    skip_store: bool,
) -> anyhow::Result<Querying<impl StorageQueryProver>>
{
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
