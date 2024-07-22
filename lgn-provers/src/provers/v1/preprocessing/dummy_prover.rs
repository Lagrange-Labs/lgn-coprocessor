use crate::provers::v1::preprocessing::prover::{
    Hash, StorageDatabaseProver, StorageExtractionProver, F,
};
use ethers::addressbook::Address;
use ethers::prelude::U256;
use std::thread::sleep;

pub(crate) struct DummyProver;

impl StorageExtractionProver for DummyProver {
    fn prove_single_variable_leaf(
        &self,
        _node: Vec<u8>,
        _slot: usize,
        _contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_single_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_mapping_variable_leaf(
        &self,
        _key: Vec<u8>,
        _node: Vec<u8>,
        _slot: usize,
        _contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_mapping_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_length_leaf(
        &self,
        _node: Vec<u8>,
        _length_slot: usize,
        _variable_slot: usize,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_length_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_contract_leaf(
        &self,
        _node: Vec<u8>,
        _storage_root: Vec<u8>,
        _contract_address: Address,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_contract_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block(&self, _rlp_header: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_final_extraction_simple(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _compound: bool,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_final_extraction_lengthed(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

impl StorageDatabaseProver for DummyProver {
    fn prove_cell_leaf(&self, _identifier: F, _value: U256) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_cell_partial(
        &self,
        _identifier: F,
        _value: U256,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_cell_full(
        &self,
        _identifier: F,
        _value: U256,
        _child_proofs: [Vec<u8>; 2],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_row_leaf(
        &self,
        _identifier: F,
        _value: U256,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_row_partial(
        &self,
        _identifier: F,
        _value: U256,
        _is_child_left: bool,
        _child_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_row_full(
        &self,
        _identifier: F,
        _value: U256,
        _left_proof: Vec<u8>,
        _right_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_membership(
        _index_identifier: F,
        _index_value: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: Hash,
        _rows_tree_hash: Hash,
        _right_child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_leaf(
        &self,
        _block_id: F,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_block_parent(
        &self,
        _block_id: F,
        _old_block_number: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: Hash,
        _right_child: Hash,
        _old_rows_tree_hash: Hash,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

#[allow(dead_code)]
fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(6000));
    let data: Vec<_> = (0..120).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
