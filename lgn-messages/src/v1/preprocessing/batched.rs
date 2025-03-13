use alloy_primitives::Address;
use alloy_primitives::U256;
use mp2_common::eth::node_type;
use mp2_common::eth::NodeType;
use mp2_common::types::HashOutput;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::BlockNr;
use crate::TableHash;
use crate::TableId;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BatchedIndex {
    pub table_id: TableId,
    pub block_nr: BlockNr,
    pub inputs: Vec<DbBlockType>,
}

impl BatchedIndex {
    pub fn new(
        table_id: TableId,
        block_nr: BlockNr,
        inputs: Vec<DbBlockType>,
    ) -> Self {
        Self {
            table_id,
            block_nr,
            inputs,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum DbBlockType {
    Leaf(BlockLeafInput),
    Parent(BlockParentInput),
    Membership(BlockMembershipInput),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockLeafInput {
    pub table_id: TableId,
    pub block_id: BlockNr,
    pub extraction_proof: Vec<u8>,
    pub rows_proof: Vec<u8>,
}

impl BlockLeafInput {
    pub fn new(
        table_id: TableId,
        block_id: BlockNr,
    ) -> Self {
        Self {
            table_id,
            block_id,
            extraction_proof: vec![],
            rows_proof: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockParentInput {
    pub table_id: TableId,
    pub block_id: BlockNr,
    pub old_block_number: U256,
    pub old_min: U256,
    pub old_max: U256,
    pub prev_left_child: Option<HashOutput>,
    pub prev_right_child: Option<HashOutput>,
    pub old_rows_tree_hash: HashOutput,
    pub extraction_proof: Vec<u8>,
    pub rows_proof: Vec<u8>,
}

impl BlockParentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        table_id: TableId,
        block_id: BlockNr,
        old_block_number: U256,
        old_min: U256,
        old_max: U256,
        prev_left_child: Option<HashOutput>,
        prev_right_child: Option<HashOutput>,
        old_rows_tree_hash: HashOutput,
    ) -> Self {
        Self {
            table_id,
            block_id,
            old_block_number,
            old_min,
            old_max,
            prev_left_child,
            prev_right_child,
            old_rows_tree_hash,
            extraction_proof: vec![],
            rows_proof: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockMembershipInput {
    pub table_id: TableId,
    pub block_id: BlockNr,
    pub index_value: U256,
    pub old_min: U256,
    pub old_max: U256,
    pub left_child: HashOutput,
    pub rows_tree_hash: HashOutput,
    pub right_proof: Vec<u8>,
}

impl BlockMembershipInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        table_id: TableId,
        block_id: BlockNr,
        index_value: U256,
        old_min: U256,
        old_max: U256,
        left_child: HashOutput,
        rows_tree_hash: HashOutput,
    ) -> Self {
        Self {
            table_id,
            block_id,
            index_value,
            old_min,
            old_max,
            left_child,
            rows_tree_hash,
            right_proof: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BatchedLength {
    pub table_hash: TableHash,
    pub block_nr: BlockNr,
    pub length_slot: usize,
    pub variable_slot: usize,
    pub nodes: Vec<Vec<u8>>,
}

impl BatchedLength {
    pub fn extraction_types(&self) -> anyhow::Result<Vec<NodeType>> {
        self.nodes.iter().map(|node| node_type(node)).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchedContract {
    pub block_nr: BlockNr,
    pub storage_root: Vec<u8>,
    pub contract: Address,
    pub nodes: Vec<Vec<u8>>,
}

impl BatchedContract {
    pub fn extraction_types(&self) -> anyhow::Result<Vec<NodeType>> {
        self.nodes.iter().map(|node| node_type(node)).collect()
    }
}
