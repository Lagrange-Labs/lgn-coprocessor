//! Groth16 prover implementation

pub trait Prover
{
    fn prove(
        &self,
        aggregated_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>>;
}
