//! This module contains logic of generating the Groth16 proofs which could be verified on-chain.
use crate::provers::v1::groth16::task::Groth16;
use prover::Prover;
use tracing::info;

mod prover;
mod task;

#[cfg(feature = "dummy-prover")]
mod dummy_prover;

#[cfg(not(feature = "dummy-prover"))]
mod euclid_prover;

#[allow(unused_variables)]
#[allow(clippy::too_many_arguments)]
pub fn create_prover(
    url: &str,
    dir: &str,
    circuit_file: &str,
    checksum_expected_local_path: &str,
    skip_checksum: bool,
    pk_file: &str,
    vk_file: &str,
    skip_store: bool,
) -> anyhow::Result<Groth16<impl Prover>> {
    let prover = {
        #[cfg(feature = "dummy-prover")]
        {
            info!("Creating dummy Groth16Prover");
            dummy_prover::DummyProver
        }
        #[cfg(not(feature = "dummy-prover"))]
        {
            info!("Creating Groth16Prover");
            euclid_prover::Groth16Prover::init(
                url,
                dir,
                circuit_file,
                checksum_expected_local_path,
                skip_checksum,
                pk_file,
                vk_file,
                skip_store,
            )?
        }
    };

    Ok(Groth16::new(prover))
}
