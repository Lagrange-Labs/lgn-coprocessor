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
    circuit_file_checksum: &str,
    pk_file: &str,
    pk_file_checksum: &str,
    vk_file: &str,
    vk_file_checksum: &str,
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
                circuit_file_checksum,
                pk_file,
                pk_file_checksum,
                vk_file,
                vk_file_checksum,
                skip_store,
            )?
        }
    };

    Ok(Groth16::new(prover))
}
