use std::fmt::Display;

use object_store::path::Path;
use serde_derive::Deserialize;
use serde_derive::Serialize;

pub(crate) const KEYS_QUERIES_PREFIX: &str = "V1_QUERIES";

type QueryId = String;

type RowKeyId = String;

type BlockNr = u64;

const ROWS_TREE: &str = "rows_tree";

const INDEX_TREE: &str = "index_tree";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey
{
    /// Initially just storing rows tree root proof
    Row(
        QueryId,
        BlockNr,
        RowKeyId,
    ),

    Index(
        QueryId,
        BlockNr,
    ),

    Revelation(QueryId),
}

impl Display for ProofKey
{
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result
    {
        match self {
            ProofKey::Row(query_id, block_nr, row_key_id) => {
                write!(
                    f,
                    "{}/{}/{ROWS_TREE}/{}/{}",
                    KEYS_QUERIES_PREFIX, query_id, block_nr, row_key_id
                )
            },
            ProofKey::Index(query_id, block_nr) => {
                write!(
                    f,
                    "{}/{}/{INDEX_TREE}/{}",
                    KEYS_QUERIES_PREFIX, query_id, block_nr
                )
            },
            ProofKey::Revelation(query_id) => {
                write!(
                    f,
                    "{}/{}/revelation",
                    KEYS_QUERIES_PREFIX, query_id
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
