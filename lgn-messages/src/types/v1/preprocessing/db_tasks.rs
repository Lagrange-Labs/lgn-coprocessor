use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use mp2_common::types::HashOutput;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::preprocessing::ext_tasks::Identifier;
use crate::BlockNr;
use crate::TableId;

#[derive(Deserialize, Serialize)]
pub enum DatabaseType {
    #[serde(rename = "1")]
    Cell {
        table_id: TableId,
        row_id: String,
        cell_id: usize,
        circuit_input: verifiable_db::cells_tree::CircuitInput,
    },

    #[serde(rename = "2")]
    Row(DbRowType),

    #[serde(rename = "3")]
    Index(IndexInputs),

    IVC(IvcInput),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum DbRowType {
    #[serde(rename = "1")]
    Leaf(RowLeafInput),

    #[serde(rename = "2")]
    Partial(RowPartialInput),

    #[serde(rename = "3")]
    Full(RowFullInput),
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct RowLeafInput {
    pub table_id: TableId,
    pub row_id: String,
    pub identifier: Identifier,
    pub value: U256,
    pub is_multiplier: bool,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct RowPartialInput {
    pub table_id: TableId,
    pub row_id: String,
    pub identifier: Identifier,
    pub value: U256,
    pub is_multiplier: bool,
    pub is_child_left: bool,

    #[dbg(placeholder = "...")]
    pub child_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct RowFullInput {
    pub table_id: TableId,
    pub row_id: String,
    pub identifier: Identifier,
    pub value: U256,
    pub is_multiplier: bool,

    #[dbg(placeholder = "...")]
    pub child_proofs: Vec<Vec<u8>>,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct IndexInputs {
    pub table_id: TableId,
    pub block_nr: BlockNr,
    pub inputs: Vec<DbBlockType>,
}

impl IndexInputs {
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

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum DbBlockType {
    #[serde(rename = "1")]
    Leaf(BlockLeafInput),

    #[serde(rename = "2")]
    Parent(BlockParentInput),

    #[serde(rename = "3")]
    Membership(BlockMembershipInput),
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockLeafInput {
    pub table_id: TableId,
    pub block_id: BlockNr,

    #[dbg(placeholder = "...")]
    pub extraction_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
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

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockParentInput {
    pub table_id: TableId,
    pub block_id: BlockNr,
    pub old_block_number: U256,
    pub old_min: U256,
    pub old_max: U256,
    pub prev_left_child: Option<HashOutput>,
    pub prev_right_child: Option<HashOutput>,
    pub old_rows_tree_hash: HashOutput,

    #[dbg(placeholder = "...")]
    pub extraction_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
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

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockMembershipInput {
    pub table_id: TableId,
    pub block_id: BlockNr,
    pub index_value: U256,
    pub old_min: U256,
    pub old_max: U256,
    pub left_child: HashOutput,
    pub rows_tree_hash: HashOutput,

    #[dbg(placeholder = "...")]
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

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct IvcInput {
    pub table_id: TableId,
    pub block_nr: BlockNr,
    pub is_first_block: bool,

    #[dbg(placeholder = "...")]
    pub index_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub previous_ivc_proof: Option<Vec<u8>>,
}

impl IvcInput {
    pub fn new(
        table_id: TableId,
        block_nr: BlockNr,
        is_first_block: bool,
    ) -> Self {
        Self {
            table_id,
            block_nr,
            is_first_block,
            index_proof: vec![],
            previous_ivc_proof: None,
        }
    }
}
