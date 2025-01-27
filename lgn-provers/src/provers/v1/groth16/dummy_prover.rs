use crate::dummy_utils::dummy_proof;
use crate::provers::v1::groth16::prover::Prover;

const PROOF_SIZE: usize = 32;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl Prover for DummyProver {
    fn prove(
        &self,
        _aggregated_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }
}
