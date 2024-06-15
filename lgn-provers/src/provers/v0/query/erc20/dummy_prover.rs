use crate::provers::v0::query::erc20::prover::QueryProver;
use ethers::addressbook::Address;
use lgn_messages::types::v0::query::erc20::{
    BlockFullNodeInput, BlockPartialNodeInput, RevelationData, StateInput, StorageBranchInput,
    StorageLeafInput,
};
use std::thread::sleep;

pub struct DummyProver;

impl QueryProver for DummyProver {
    fn prove_storage_leaf(
        &self,
        contract: Address,
        data: &StorageLeafInput,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_storage_branch(&self, data: &StorageBranchInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db(&self, contract: Address, data: &StateInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_partial_node(&self, data: &BlockPartialNodeInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_full_node(&self, data: &BlockFullNodeInput) -> anyhow::Result<Vec<u8>> {
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
