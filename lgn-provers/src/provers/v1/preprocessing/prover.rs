use alloy::primitives::U256;
use mp2_common::digest::TableDimension;
use mp2_common::types::HashOutput;
use mp2_v1::contract_extraction;
use mp2_v1::length_extraction;
use mp2_v1::values_extraction;

pub trait StorageExtractionProver {
    /// Prove a value extraction
    fn prove_value_extraction(
        &self,
        circuit_input: values_extraction::CircuitInput,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a length extraction.
    fn prove_length_extraction(
        &self,
        circuit_input: length_extraction::LengthCircuitInput,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a contract extraction.
    fn prove_contract_extraction(
        &self,
        circuit_input: contract_extraction::CircuitInput,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a block.
    /// TODO: implement this
    fn prove_block(
        &self,
        rlp_header: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove final extraction for simple types
    fn prove_final_extraction_simple(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        value_proof: Vec<u8>,
        dimension: TableDimension,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove final extraction for lengthed types
    fn prove_final_extraction_lengthed(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        value_proof: Vec<u8>,
        length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove final extraction for merge types
    fn prove_final_extraction_merge(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        simple_table_proof: Vec<u8>,
        mapping_table_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;
}

pub trait StorageDatabaseProver {
    /// Prove a cell tree leaf node.
    fn prove_cell_leaf(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a cell tree partial branch node.
    fn prove_cell_partial(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a cell tree full branch node.
    fn prove_cell_full(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree leaf node.
    fn prove_row_leaf(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree partial branch node.
    fn prove_row_partial(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        is_child_left: bool,
        child_proof: Vec<u8>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree full branch node.
    fn prove_row_full(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proofs: Vec<Vec<u8>>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Create a circuit input for proving a leaf node.
    fn prove_block_leaf(
        &self,
        block_id: u64,
        extraction_proof: Vec<u8>,
        rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Create a circuit input for proving a parent node.
    #[allow(clippy::too_many_arguments)]
    fn prove_block_parent(
        &self,
        block_id: u64,
        old_block_number: U256,
        old_min: U256,
        old_max: U256,
        old_left_child: Option<HashOutput>,
        old_right_child: Option<HashOutput>,
        old_rows_tree_hash: HashOutput,
        extraction_proof: Vec<u8>,
        rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Create a circuit input for proving a membership node of 1 child.
    #[allow(clippy::too_many_arguments)]
    fn prove_membership(
        &self,
        block_id: u64,
        index_value: U256,
        old_min: U256,
        old_max: U256,
        left_child: HashOutput,
        rows_tree_hash: HashOutput,
        right_child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_ivc(
        &self,
        index_proof: Vec<u8>,
        previous_proof: Option<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>;
}
