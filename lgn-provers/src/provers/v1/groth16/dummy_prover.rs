use crate::provers::v1::groth16::prover::Prover;
use std::thread::sleep;

#[allow(dead_code)]
pub(crate) struct DummyProver;

impl Prover for DummyProver {
    fn prove(&self, _aggregated_proof: &[u8]) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

#[allow(dead_code)]
fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(1000));
    let data: Vec<_> = (0..32).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
