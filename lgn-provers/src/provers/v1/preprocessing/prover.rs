use alloy::primitives::Address;
use alloy::primitives::U256;
use mp2_common::digest::TableDimension;
use mp2_common::types::HashOutput;

pub trait StorageExtractionProver
{
    /// Prove a leaf MPT node of single variable.
    fn prove_single_variable_leaf(
        &self,
        node: Vec<u8>,
        slot: u8,
        column_id: u64,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a branch MPT node of single variable.
    fn prove_single_variable_branch(
        &self,
        node: Vec<u8>,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_mapping_variable_leaf(
        &self,
        key: Vec<u8>,
        node: Vec<u8>,
        slot: u8,
        key_id: u64,
        value_id: u64,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a branch MPT node of mapping variable.
    fn prove_mapping_variable_branch(
        &self,
        node: Vec<u8>,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove the length extraction of a leaf MPT node.
    fn prove_length_leaf(
        &self,
        node: Vec<u8>,
        length_slot: usize,
        variable_slot: usize,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove the length extraction of a branch MPT node.
    fn prove_length_branch(
        &self,
        node: Vec<u8>,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a leaf MPT node of contract.
    fn prove_contract_leaf(
        &self,
        node: Vec<u8>,
        storage_root: Vec<u8>,
        contract_address: Address,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a branch MPT node of contract.
    fn prove_contract_branch(
        &self,
        node: Vec<u8>,
        child_proof: Vec<u8>,
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

pub trait StorageDatabaseProver
{
    /// Prove a cell tree leaf node.
    fn prove_cell_leaf(&self, identifier: u64, value: U256) -> anyhow::Result<Vec<u8>>;

    /// Prove a cell tree partial branch node.
    fn prove_cell_partial(
        &self,
        identifier: u64,
        value: U256,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a cell tree full branch node.
    fn prove_cell_full(
        &self,
        identifier: u64,
        value: U256,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree leaf node.
    fn prove_row_leaf(
        &self,
        identifier: u64,
        value: U256,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree partial branch node.
    fn prove_row_partial(
        &self,
        identifier: u64,
        value: U256,
        is_child_left: bool,
        child_proof: Vec<u8>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    /// Prove a row tree full branch node.
    fn prove_row_full(
        &self,
        identifier: u64,
        value: U256,
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
