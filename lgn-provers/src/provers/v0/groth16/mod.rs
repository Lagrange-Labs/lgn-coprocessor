//! This module contains logic of generating the Groth16 proofs which could be verified on-chain.

use self::prover::Prover;
use crate::provers::v0::groth16::task::Groth16;
use tracing::info;

mod dummy_prover;
mod prover;
mod task;

#[allow(unused_variables)]
pub fn create_prover(
    url: &str,
    dir: &str,
    circuit_file: &str,
    checksum_expected_local_path: &str,
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
            prover::Groth16Prover::init(
                url,
                dir,
                circuit_file,
                checksum_expected_local_path,
                pk_file,
                vk_file,
                skip_store,
            )?
        }
    };

    Ok(Groth16::new(prover))
}
