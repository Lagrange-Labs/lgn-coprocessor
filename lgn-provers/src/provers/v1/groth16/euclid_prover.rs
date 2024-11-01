use anyhow::Result;
use groth16_framework_v1::Groth16Prover as InnerProver;
use tracing::debug;

use crate::params::ParamsLoader;
use crate::provers::v1::groth16::prover::Prover;

#[derive(Debug)]
pub struct Groth16Prover
{
    inner: InnerProver,
}

impl Groth16Prover
{
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        url: &str,
        dir: &str,
        circuit_file: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        r1cs_file: &str,
        pk_file: &str,
        skip_store: bool,
    ) -> Result<Self>
    {
        let circuit_bytes = ParamsLoader::prepare_raw(
            url,
            dir,
            circuit_file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )?;
        let r1cs_bytes = ParamsLoader::prepare_raw(
            url,
            dir,
            r1cs_file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )?;
        let pk_bytes = ParamsLoader::prepare_raw(
            url,
            dir,
            pk_file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )?;

        debug!("Creating Groth16 prover");
        let inner = InnerProver::from_bytes(
            r1cs_bytes.to_vec(),
            pk_bytes.to_vec(),
            circuit_bytes.to_vec(),
        )?;

        debug!("Groth16 prover created");
        Ok(
            Self {
                inner,
            },
        )
    }
}

impl Prover for Groth16Prover
{
    fn prove(
        &self,
        revelation: &[u8],
    ) -> Result<Vec<u8>>
    {
        self.inner
            .prove(revelation)
    }
}
