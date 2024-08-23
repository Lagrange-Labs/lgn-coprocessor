use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use mp2_common::types::HashOutput;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::types::v1::preprocessing::ext_tasks::{Identifier, WorkerTask};
use crate::types::v1::preprocessing::{db_keys, ext_keys, WorkerTaskType};

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub enum DatabaseType {
    #[serde(rename = "1")]
    Cell(DbCellType),

    #[serde(rename = "2")]
    Row(DbRowType),

    #[serde(rename = "3")]
    Index(IndexInputs),

    IVC(IvcInput),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum DbCellType {
    #[serde(rename = "1")]
    Leaf(CellLeafInput),

    #[serde(rename = "2")]
    Partial(CellPartialInput),

    #[serde(rename = "3")]
    Full(CellFullInput),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct CellLeafInput {
    pub table_id: u64,

    pub row_id: String,

    pub cell_id: usize,

    pub identifier: Identifier,

    pub value: U256,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct CellPartialInput {
    pub table_id: u64,

    pub row_id: String,

    pub cell_id: usize,

    pub identifier: Identifier,

    pub value: U256,

    pub child_location: db_keys::ProofKey,

    #[dbg(placeholder = "...")]
    pub child_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct CellFullInput {
    pub table_id: u64,

    pub row_id: String,

    pub cell_id: usize,

    pub identifier: Identifier,

    pub value: U256,

    pub child_locations: Vec<db_keys::ProofKey>,

    #[dbg(placeholder = "...")]
    pub child_proofs: Vec<Vec<u8>>,
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
    pub table_id: u64,

    pub row_id: String,

    pub identifier: Identifier,

    pub value: U256,

    pub cells_proof_location: Option<db_keys::ProofKey>,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct RowPartialInput {
    pub table_id: u64,

    pub row_id: String,

    pub identifier: Identifier,

    pub value: U256,

    pub is_child_left: bool,

    pub child_proof_location: db_keys::ProofKey,

    pub cells_proof_location: Option<db_keys::ProofKey>,

    #[dbg(placeholder = "...")]
    pub child_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct RowFullInput {
    pub table_id: u64,

    pub row_id: String,

    pub identifier: Identifier,

    pub value: U256,

    pub child_proofs_locations: Vec<db_keys::ProofKey>,

    pub cells_proof_location: Option<db_keys::ProofKey>,

    #[dbg(placeholder = "...")]
    pub child_proofs: Vec<Vec<u8>>,

    #[dbg(placeholder = "...")]
    pub cells_proof: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct IndexInputs {
    pub table_id: u64,

    pub block_nr: u64,

    pub inputs: Vec<DbBlockType>,
}

impl IndexInputs {
    pub fn new(table_id: u64, block_nr: u64, inputs: Vec<DbBlockType>) -> Self {
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
    pub table_id: u64,

    pub block_id: u64,

    pub extraction_proof_location: ext_keys::ProofKey,

    pub rows_proof_location: db_keys::ProofKey,

    #[dbg(placeholder = "...")]
    pub extraction_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub rows_proof: Vec<u8>,
}

impl BlockLeafInput {
    pub fn new(
        table_id: u64,
        block_id: u64,
        extraction_proof_location: ext_keys::ProofKey,
        rows_proof_location: db_keys::ProofKey,
    ) -> Self {
        Self {
            table_id,
            block_id,
            extraction_proof_location,
            rows_proof_location,
            extraction_proof: vec![],
            rows_proof: vec![],
        }
    }
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockParentInput {
    pub table_id: u64,

    pub block_id: u64,

    pub old_block_number: U256,

    pub old_min: U256,

    pub old_max: U256,

    pub prev_left_child: Option<HashOutput>,

    pub prev_right_child: Option<HashOutput>,

    pub old_rows_tree_hash: HashOutput,

    pub extraction_proof_location: ext_keys::ProofKey,

    pub rows_proof_location: db_keys::ProofKey,

    #[dbg(placeholder = "...")]
    pub extraction_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub rows_proof: Vec<u8>,
}

impl BlockParentInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        table_id: u64,
        block_id: u64,
        old_block_number: U256,
        old_min: U256,
        old_max: U256,
        prev_left_child: Option<HashOutput>,
        prev_right_child: Option<HashOutput>,
        old_rows_tree_hash: HashOutput,
        extraction_proof_location: ext_keys::ProofKey,
        rows_proof_location: db_keys::ProofKey,
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
            extraction_proof_location,
            rows_proof_location,
            extraction_proof: vec![],
            rows_proof: vec![],
        }
    }
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockMembershipInput {
    pub table_id: u64,

    pub block_id: u64,

    pub index_value: U256,

    pub old_min: U256,

    pub old_max: U256,

    pub left_child: HashOutput,

    pub rows_tree_hash: HashOutput,

    pub right_proof_location: db_keys::ProofKey,

    #[dbg(placeholder = "...")]
    pub right_proof: Vec<u8>,
}

impl BlockMembershipInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        table_id: u64,
        block_id: u64,
        index_value: U256,
        old_min: U256,
        old_max: U256,
        left_child: HashOutput,
        rows_tree_hash: HashOutput,
        right_proof_location: db_keys::ProofKey,
    ) -> Self {
        Self {
            table_id,
            block_id,
            index_value,
            old_min,
            old_max,
            left_child,
            rows_tree_hash,
            right_proof_location,
            right_proof: vec![],
        }
    }
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct IvcInput {
    pub table_id: u64,

    pub block_nr: u64,

    pub is_first_block: bool,

    #[dbg(placeholder = "...")]
    pub index_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub previous_ivc_proof: Option<Vec<u8>>,
}

impl IvcInput {
    pub fn new(table_id: u64, block_nr: u64, is_first_block: bool) -> Self {
        Self {
            table_id,
            block_nr,
            is_first_block,
            index_proof: vec![],
            previous_ivc_proof: None,
        }
    }
}

impl From<&WorkerTask> for db_keys::ProofKey {
    fn from(tt: &WorkerTask) -> Self {
        match &tt.task_type {
            WorkerTaskType::Database(db) => match db {
                DatabaseType::Cell(ct) => match ct {
                    DbCellType::Leaf(cl) => db_keys::ProofKey::Cell(
                        cl.table_id,
                        tt.block_nr,
                        cl.row_id.to_owned(),
                        cl.cell_id,
                    ),
                    DbCellType::Partial(cp) => db_keys::ProofKey::Cell(
                        cp.table_id,
                        tt.block_nr,
                        cp.row_id.to_owned(),
                        cp.cell_id,
                    ),
                    DbCellType::Full(cf) => db_keys::ProofKey::Cell(
                        cf.table_id,
                        tt.block_nr,
                        cf.row_id.to_owned(),
                        cf.cell_id,
                    ),
                },
                DatabaseType::Row(rt) => match rt {
                    DbRowType::Leaf(rl) => {
                        db_keys::ProofKey::Row(rl.table_id, tt.block_nr, rl.row_id.to_string())
                    }
                    DbRowType::Partial(rp) => {
                        db_keys::ProofKey::Row(rp.table_id, tt.block_nr, rp.row_id.to_string())
                    }
                    DbRowType::Full(rf) => {
                        db_keys::ProofKey::Row(rf.table_id, tt.block_nr, rf.row_id.to_string())
                    }
                },
                DatabaseType::Index(bt) => db_keys::ProofKey::Block(bt.table_id, tt.block_nr),
                DatabaseType::IVC(ivc) => db_keys::ProofKey::IVC(ivc.table_id, tt.block_nr),
            },
            _ => unimplemented!("Task type not supported: {:?}", tt.task_type),
        }
    }
}
