use crate::types::v1::query::tasks::QueryInput;
use crate::types::v1::query::KEYS_QUERIES_PREFIX;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

type QueryId = String;

type RowKeyId = String;

type BlockNr = u64;

type IndexNodeId = usize;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    Row(QueryId, BlockNr, RowKeyId),

    Index(QueryId, BlockNr, IndexNodeId),

    Revelation(QueryId),
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::Row(query_id, block_nr, row_key_id) => {
                write!(
                    f,
                    "{}/{}/{}/{}",
                    KEYS_QUERIES_PREFIX, query_id, block_nr, row_key_id
                )
            }
            ProofKey::Index(query_id, block_nr, index_node_id) => {
                write!(
                    f,
                    "{}/{}/{}/{}",
                    KEYS_QUERIES_PREFIX, query_id, block_nr, index_node_id
                )
            }
            ProofKey::Revelation(query_id) => {
                write!(f, "{}/{}/revelation", KEYS_QUERIES_PREFIX, query_id)
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
