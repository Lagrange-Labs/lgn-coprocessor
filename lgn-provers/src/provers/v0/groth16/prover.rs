//! Groth16 prover implementation

use crate::params::ParamsLoader;
use anyhow::Result;
use groth16_framework::Groth16Prover as InnerProver;
use tracing::debug;

pub trait Prover {
    fn prove(&self, aggregated_proof: &[u8]) -> Result<Vec<u8>>;
}

#[derive(Debug)]
pub struct Groth16Prover {
    inner: InnerProver,
}

impl Groth16Prover {
    // #[allow(dead_code)] - clippy warning because of dummy-prover feature
    #[allow(dead_code)]
    pub fn init(
        url: &str,
        dir: &str,
        checksum_expected_local_path: &str,
        circuit_file: &str,
        r1cs_file: &str,
        pk_file: &str,
        skip_store: bool,
    ) -> Result<Self> {
        let circuit_bytes = ParamsLoader::prepare_raw(
            url,
            dir,
            checksum_expected_local_path,
            circuit_file,
            skip_store,
        )?;
        let r1cs_bytes = ParamsLoader::prepare_raw(
            url,
            dir,
            checksum_expected_local_path,
            r1cs_file,
            skip_store,
        )?;
        let pk_bytes =
            ParamsLoader::prepare_raw(url, dir, checksum_expected_local_path, pk_file, skip_store)?;

        debug!("Creating Groth16 prover");
        let inner = InnerProver::from_bytes(
            r1cs_bytes.to_vec(),
            pk_bytes.to_vec(),
            circuit_bytes.to_vec(),
        )?;

        debug!("Groth16 prover created");
        Ok(Self { inner })
    }
}

impl Prover for Groth16Prover {
    fn prove(&self, aggregated_proof: &[u8]) -> Result<Vec<u8>> {
        self.inner.prove(aggregated_proof)
    }
}
