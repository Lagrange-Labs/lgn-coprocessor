use std::fmt::Display;

use object_store::path::Path;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::preprocessing::KEYS_PREPROCESSING_PREFIX;
use crate::BlockNr;
use crate::TableId;

type RowId = String;
type CellId = usize;

const CELL_PREFIX: &str = "CELL";
const ROW_PREFIX: &str = "ROW";
const BLOCK_PREFIX: &str = "DB_BLOCK";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey
{
    /// Indicates the location of Cell proof.
    Cell(
        TableId,
        BlockNr,
        RowId,
        CellId,
    ),

    /// Indicates the location of Row proof.
    Row(
        TableId,
        BlockNr,
        RowId,
    ),

    Block(
        TableId,
        BlockNr,
    ),

    IVC(
        TableId,
        BlockNr,
    ),
}

impl Display for ProofKey
{
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result
    {
        match self
        {
            ProofKey::Cell(table_id, block_nr, row_id, cell_id) =>
            {
                // Example: V1_PREPROCESSING/CELL/1/2/3/4/5
                write!(f, "{KEYS_PREPROCESSING_PREFIX}/{table_id}/{block_nr}/{CELL_PREFIX}/{row_id}/{cell_id}")
            },
            ProofKey::Row(table_id, block_nr, row_id) =>
            {
                // Example: V1_PREPROCESSING/ROW/1/2/3/4
                write!(
                    f,
                    "{KEYS_PREPROCESSING_PREFIX}/{table_id}/{block_nr}/{ROW_PREFIX}/{row_id}"
                )
            },
            ProofKey::Block(table_id, block_nr) =>
            {
                // Example: V1_PREPROCESSING/DB_BLOCK/1/2
                write!(
                    f,
                    "{KEYS_PREPROCESSING_PREFIX}/{BLOCK_PREFIX}/{table_id}/{block_nr}"
                )
            },
            ProofKey::IVC(table_id, block_nr) =>
            {
                // Example: V1_PREPROCESSING/IVC/1/2
                write!(
                    f,
                    "{KEYS_PREPROCESSING_PREFIX}/IVC/{table_id}/{block_nr}"
                )
            },
        }
    }
}

impl From<ProofKey> for Path
{
    fn from(key: ProofKey) -> Self
    {
        Path::from(key.to_string())
    }
}

impl From<ProofKey> for String
{
    fn from(key: ProofKey) -> Self
    {
        key.to_string()
    }
}
