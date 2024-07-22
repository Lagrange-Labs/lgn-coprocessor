use crate::provers::v1::preprocessing::prover::{StorageDatabaseProver, StorageExtractionProver};
use alloy::primitives::{Address, U256};
use mp2_common::types::HashOutput;
use std::thread::sleep;
use tracing::debug;

#[allow(dead_code)]
pub(crate) struct DummyProver;

impl StorageExtractionProver for DummyProver {
    fn prove_single_variable_leaf(
        &self,
        _node: Vec<u8>,
        _slot: usize,
        _contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving single variable leaf");
        Ok(prove())
    }

    fn prove_single_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving single variable branch");
        Ok(prove())
    }

    fn prove_mapping_variable_leaf(
        &self,
        _key: Vec<u8>,
        _node: Vec<u8>,
        _slot: usize,
        _contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving mapping variable leaf");
        Ok(prove())
    }

    fn prove_mapping_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving mapping variable branch");
        Ok(prove())
    }

    fn prove_length_leaf(
        &self,
        _node: Vec<u8>,
        _length_slot: usize,
        _variable_slot: usize,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving length leaf");
        Ok(prove())
    }

    fn prove_length_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving length branch");
        Ok(prove())
    }

    fn prove_contract_leaf(
        &self,
        _node: Vec<u8>,
        _storage_root: Vec<u8>,
        _contract_address: Address,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving contract leaf");
        Ok(prove())
    }

    fn prove_contract_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving contract branch");
        Ok(prove())
    }

    fn prove_block(&self, _rlp_header: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        debug!("Proving block");
        Ok(prove())
    }

    fn prove_final_extraction_simple(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _compound: bool,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving final extraction simple");
        Ok(prove())
    }

    fn prove_final_extraction_lengthed(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving final extraction lengthed");
        Ok(prove())
    }
}

impl StorageDatabaseProver for DummyProver {
    fn prove_cell_leaf(&self, _identifier: u64, _value: U256) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_cell_partial(
        &self,
        _identifier: u64,
        _value: U256,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving cell partial");
        Ok(prove())
    }

    fn prove_cell_full(
        &self,
        _identifier: u64,
        _value: U256,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving cell full");
        Ok(prove())
    }

    fn prove_row_leaf(
        &self,
        _identifier: u64,
        _value: U256,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row leaf");
        Ok(prove())
    }

    fn prove_row_partial(
        &self,
        _identifier: u64,
        _value: U256,
        _is_child_left: bool,
        _child_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row partial");
        Ok(prove())
    }

    fn prove_row_full(
        &self,
        _identifier: u64,
        _value: U256,
        _child_proofs: Vec<Vec<u8>>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row full");
        Ok(prove())
    }

    fn prove_membership(
        &self,
        _block_id: u64,
        _index_value: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: HashOutput,
        _rows_tree_hash: HashOutput,
        _right_child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving membership");
        Ok(prove())
    }

    fn prove_block_leaf(
        &self,
        _block_id: u64,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving block leaf");
        Ok(prove())
    }

    fn prove_block_parent(
        &self,
        _block_id: u64,
        _old_block_number: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: HashOutput,
        _right_child: HashOutput,
        _old_rows_tree_hash: HashOutput,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving block parent");
        Ok(prove())
    }
}

#[allow(dead_code)]
fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(100));
    let data: Vec<_> = (0..120).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
