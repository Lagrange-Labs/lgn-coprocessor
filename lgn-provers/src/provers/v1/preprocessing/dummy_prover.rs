use alloy::primitives::U256;
use mp2_common::digest::TableDimension;
use mp2_common::types::HashOutput;
use mp2_v1::contract_extraction;
use mp2_v1::length_extraction;
use mp2_v1::values_extraction;
use tracing::debug;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::preprocessing::prover::PreprocessingProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl PreprocessingProver for DummyProver {
    fn prove_value_extraction(
        &self,
        _circuit_input: values_extraction::CircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving value extraction");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_length_extraction(
        &self,
        _circuit_input: length_extraction::LengthCircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving length extraction");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_contract_extraction(
        &self,
        _circuit_input: contract_extraction::CircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving contract extraction");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_block(
        &self,
        _rlp_header: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving block");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_simple(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _dimension: TableDimension,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving final extraction simple");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_lengthed(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _value_proof: Vec<u8>,
        _length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving final extraction lengthed");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_final_extraction_merge(
        &self,
        _block_proof: Vec<u8>,
        _contract_proof: Vec<u8>,
        _simple_table_proof: Vec<u8>,
        _mapping_table_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving final extraction merge table");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_cells_tree(
        &self,
        _circuit_input: verifiable_db::cells_tree::CircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving cells tree");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_row_leaf(
        &self,
        _identifier: u64,
        _value: U256,
        _is_multiplier: bool,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
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
    ) -> anyhow::Result<Vec<u8>> {
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
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row full");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_block_leaf(
        &self,
        _block_id: u64,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
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
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving block parent");
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
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving membership");
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_ivc(
        &self,
        _block_proof: Vec<u8>,
        _previous_proof: Option<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving ivc");
        Ok(dummy_proof(PROOF_SIZE))
    }
}
