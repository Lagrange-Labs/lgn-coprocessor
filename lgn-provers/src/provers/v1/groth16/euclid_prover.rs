use std::collections::HashMap;

use anyhow::Result;
use tracing::info;

use crate::params;
use crate::provers::v1::groth16::prover::Prover;

#[derive(Debug)]
pub struct Groth16Prover {
    inner: groth16_framework_v1::Groth16Prover,
}

impl Groth16Prover {
    /// Initialize the Groth16 prover from bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        url: &str,
        dir: &str,
        circuit_file: &str,
        r1cs_file: &str,
        pk_file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> Result<Self> {
        let circuit_bytes = params::prepare_raw(url, dir, circuit_file, checksums)?;
        let r1cs_bytes = params::prepare_raw(url, dir, r1cs_file, checksums)?;
        let pk_bytes = params::prepare_raw(url, dir, pk_file, checksums)?;

        info!("Creating Groth16 prover");
        let inner = groth16_framework_v1::Groth16Prover::from_bytes(
            r1cs_bytes.to_vec(),
            pk_bytes.to_vec(),
            circuit_bytes.to_vec(),
        )?;
        info!("Groth16 prover created");

        Ok(Self { inner })
    }
}

impl Prover for Groth16Prover {
    /// Generate the Groth16 proof from the plonky2 proof.
    fn prove(
        &self,
        revelation: &[u8],
    ) -> Result<Vec<u8>> {
        self.inner.prove(revelation)
    }
}
