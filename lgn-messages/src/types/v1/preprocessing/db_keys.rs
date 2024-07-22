use crate::types::v1::preprocessing::KEYS_PREPROCESSING_PREFIX;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

type BlockNr = u64;

type TableId = u64;

type RowId = String;

type CellId = usize;

const CELL_PREFIX: &str = "CELL";

const ROW_PREFIX: &str = "ROW";

const BLOCK_PREFIX: &str = "BLOCK";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Indicates the location of Cell proof.
    Cell(TableId, BlockNr, RowId, CellId),

    /// Indicates the location of Row proof.
    Row(TableId, BlockNr, RowId),

    Block(TableId, BlockNr),
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::Cell(table_id, block_nr, row_id, cell_id) => {
                // Example: V1_PREPROCESSING_CELL_1_2_3_4
                write!(f, "{KEYS_PREPROCESSING_PREFIX}_{table_id}_{block_nr}_{CELL_PREFIX}_{row_id}_{cell_id}")
            }
            ProofKey::Row(table_id, block_nr, row_id) => {
                // Example: V1_PREPROCESSING_ROW_1_2_3
                write!(
                    f,
                    "{KEYS_PREPROCESSING_PREFIX}_{table_id}_{block_nr}_{ROW_PREFIX}_{row_id}"
                )
            }
            ProofKey::Block(table_id, block_nr) => {
                // Example: V1_PREPROCESSING_BLOCK_1_2
                write!(
                    f,
                    "{KEYS_PREPROCESSING_PREFIX}_{BLOCK_PREFIX}_{table_id}_{block_nr}"
                )
            }
        }
    }
}

impl From<ProofKey> for Path {
    fn from(key: ProofKey) -> Self {
        Path::from(key.to_string())
    }
}

impl From<ProofKey> for String {
    fn from(key: ProofKey) -> Self {
        key.to_string()
    }
}
