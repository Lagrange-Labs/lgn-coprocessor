use alloy::primitives::Address;
use alloy::primitives::U256;
use mp2_common::digest::TableDimension;
use mp2_common::types::HashOutput;
use tracing::debug;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::preprocessing::prover::StorageDatabaseProver;
use crate::provers::v1::preprocessing::prover::StorageExtractionProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl StorageExtractionProver for DummyProver
{
    fn prove_single_variable_leaf(
        &self,
        _node: Vec<u8>,
        _slot: u8,
        _column_id: u64,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving single variable leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_single_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving single variable branch");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_mapping_variable_leaf(
        &self,
        _key: Vec<u8>,
        _node: Vec<u8>,
        _slot: u8,
        _key_id: u64,
        _value_id: u64,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving mapping variable leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_mapping_variable_branch(
        &self,
        _node: Vec<u8>,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving mapping variable branch");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_length_leaf(
        &self,
        _node: Vec<u8>,
        _length_slot: usize,
        _variable_slot: usize,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving length leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_length_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving length branch");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_contract_leaf(
        &self,
        _node: Vec<u8>,
        _storage_root: Vec<u8>,
        _contract_address: Address,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving contract leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_contract_branch(
        &self,
        _node: Vec<u8>,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving contract branch");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_block(
        &self,
        _rlp_header: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving block");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_simple(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _dimension: TableDimension,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving final extraction simple");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_lengthed(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving final extraction lengthed");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_merge(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _simple_table_proof: Vec<u8>,
        _mapping_table_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving final extraction merge table");
        Ok(dummy_proof(PROOF_SIZE))
    }
}

impl StorageDatabaseProver for DummyProver
{
    fn prove_cell_leaf(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
    ) -> anyhow::Result<Vec<u8>>
    {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_cell_partial(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving cell partial");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_cell_full(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving cell full");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_row_leaf(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving row leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_row_partial(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _is_child_left: bool,
        _child_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving row partial");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_row_full(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _child_proofs: Vec<Vec<u8>>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving row full");
        Ok(dummy_proof(PROOF_SIZE))
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
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving membership");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_block_leaf(
        &self,
        _block_id: u64,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving block leaf");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_block_parent(
        &self,
        _block_id: u64,
        _old_block_number: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: Option<HashOutput>,
        _right_child: Option<HashOutput>,
        _old_rows_tree_hash: HashOutput,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving block parent");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_ivc(
        &self,
        _block_proof: Vec<u8>,
        _previous_proof: Option<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>
    {
        debug!("Proving ivc");
        Ok(dummy_proof(PROOF_SIZE))
    }
}
