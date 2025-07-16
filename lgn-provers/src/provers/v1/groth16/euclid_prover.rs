use std::collections::HashMap;
use std::fs::read;

use anyhow::Context;
use anyhow::Result;
use groth16_framework::Groth16Prover;
use tracing::debug;

use crate::params;

#[derive(Debug)]
pub struct Groth16EuclidProver {
    inner: Groth16Prover,
}

impl Groth16EuclidProver {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn init(
        url: &str,
        dir: &str,
        circuit_file: &str,
        r1cs_file: &str,
        pk_file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> Result<Self> {
        let circuit_bytes_path =
            params::download_and_checksum(url, dir, circuit_file, checksums).await?;
        let r1cs_bytes_path = params::download_and_checksum(url, dir, r1cs_file, checksums).await?;
        let pk_bytes_path = params::download_and_checksum(url, dir, pk_file, checksums).await?;

        let r1cs = read(&r1cs_bytes_path)
            .with_context(|| format!("while reading {}", r1cs_bytes_path.display()))?;
        let pk = read(&pk_bytes_path)
            .with_context(|| format!("while reading {}", pk_bytes_path.display()))?;
        let circuit = read(&circuit_bytes_path)
            .with_context(|| format!("while reading {}", circuit_bytes_path.display()))?;

        debug!("Creating Groth16 prover");
        let inner = Groth16Prover::from_bytes(r1cs, pk, circuit)?;

        debug!("Groth16 prover created");
        Ok(Self { inner })
    }

    pub(super) fn prove(
        &self,
        revelation: &[u8],
    ) -> Result<Vec<u8>> {
        self.inner.prove(revelation)
    }
}
