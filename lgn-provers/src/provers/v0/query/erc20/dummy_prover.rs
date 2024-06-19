use std::thread::sleep;

use ethers::addressbook::Address;

use lgn_messages::types::v0::query::erc20::{RevelationData, StorageBranchInput, StorageLeafInput};
use lgn_messages::types::v0::query::{PartialNodeBlockData, QueryStateData};

use crate::provers::v0::query::erc20::prover::QueryProver;

pub struct DummyProver;

impl QueryProver for DummyProver {
    fn prove_storage_leaf(&self, _data: &StorageLeafInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_storage_branch(&self, _data: &StorageBranchInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db(
        &self,
        _contract: Address,
        _data: &QueryStateData,
    ) -> anyhow::Result<Vec<u8>> {
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
