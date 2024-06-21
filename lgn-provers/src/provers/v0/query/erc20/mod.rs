use crate::provers::v0::query::erc20::prover::QueryProver;
use crate::provers::v0::query::erc20::task::Query;
use tracing::info;

mod dummy_prover;
mod prover;
mod task;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    file: &str,
    checksum_expected_local_path: &str,
    skip_checksum: bool,
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
            use crate::provers::v0::query::erc20::prover::EuclidProver;

            info!("Creating query prover");
            EuclidProver::init(
                url,
                dir,
                file,
                checksum_expected_local_path,
                skip_checksum,
                skip_store,
            )?
        }
    };

    Ok(Query::new(prover))
}
