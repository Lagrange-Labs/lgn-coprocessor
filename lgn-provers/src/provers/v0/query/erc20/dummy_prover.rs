use crate::provers::v0::query::erc20::prover::QueryProver;
use ethers::addressbook::Address;
use lgn_messages::types::v0::query::erc20::{
    RevelationData, StorageBranchInput,
    StorageLeafInput,
};
use std::thread::sleep;
use lgn_messages::types::v0::query::{FullNodeBlockData, PartialNodeBlockData, QueryStateData};

pub struct DummyProver;

impl QueryProver for DummyProver {
    fn prove_storage_leaf(
        &self,
        data: &StorageLeafInput,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_storage_branch(&self, data: &StorageBranchInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db(&self, contract: Address, data: &QueryStateData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_partial_node(&self, data: &PartialNodeBlockData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_full_node(
        &self,
        left_proof: &[u8],
        right_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_revelation(&self, data: &RevelationData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(1000));
    let data: Vec<_> = (0..32).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
