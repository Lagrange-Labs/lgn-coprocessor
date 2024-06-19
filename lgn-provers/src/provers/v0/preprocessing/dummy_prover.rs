use crate::provers::v0::preprocessing::prover::StorageProver;
use lgn_messages::types::v0::preprocessing::{
    BlockLinkingInput, BlocksDbData, LengthExtractInput, MptProofBranchData, MptProofLeafData,
    StorageDbLeafData,
};
use std::thread::sleep;

#[allow(dead_code)]
pub(crate) struct DummyProver;

impl StorageProver for DummyProver {
    fn prove_mpt_leaf(&self, _data: &MptProofLeafData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_mpt_branch(&self, _data: &MptProofBranchData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_storage_db_leaf(&self, _data: StorageDbLeafData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_storage_db_branch(
        &self,
        _left_proof: Vec<u8>,
        _right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db_leaf(&self, _block_linking_proof: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_state_db_branch(
        &self,
        _left_proof: Vec<u8>,
        _right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_length_extract(&self, _data: LengthExtractInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_length_match(
        &self,
        _mapping_proof: &[u8],
        _length_extract_proof: &[u8],
        _skip_match: bool,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_equivalence(
        &self,
        _lpn_proof: Vec<u8>,
        _mpt_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_number_linking(&self, _data: &BlockLinkingInput) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_blocks_db_first(&self, _block_leaf_index: BlocksDbData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_blocks_db_subsequent(&self, _data: BlocksDbData) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

#[allow(dead_code)]
fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(6000));
    let data: Vec<_> = (0..120).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
