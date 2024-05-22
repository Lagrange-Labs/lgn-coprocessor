use crate::provers::v0::preprocessing::prover::StorageProver;
use crate::provers::v0::preprocessing::task::Preprocessing;
use tracing::info;

mod dummy_prover;
pub(crate) mod prover;
pub(crate) mod task;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    skip_store: bool,
) -> anyhow::Result<Preprocessing<impl StorageProver>> {
    let prover = {
        #[cfg(feature = "dummy-prover")]
        {
            info!("Creating dummy storage prover");
            dummy_prover::DummyProver
        }

        #[cfg(not(feature = "dummy-prover"))]
        {
            info!("Creating storage prover");
            prover::RealStorageProver::init(url, dir, file, skip_store)?
        }
    };

    Ok(Preprocessing::new(prover))
}
