use crate::provers::v0::query::erc721::prover::QueryProver;
use lgn_messages::types::v0::query::{
    PartialNodeBlockData, QueryStateData, RevelationData, StorageProofInput,
};
use std::thread::sleep;

pub struct DummyProver;

impl QueryProver for DummyProver {
    fn prove_storage_entry(&self, _data: StorageProofInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db(&self, _d: &QueryStateData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_partial_node(&self, _data: &PartialNodeBlockData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_full_node(
        &self,
        _left_proof: &[u8],
        _right_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_revelation(&self, _data: &RevelationData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(1000));
    let data: Vec<_> = (0..32).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
