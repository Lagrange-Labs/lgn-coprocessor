use std::fmt::Display;

use mp2_v1::query::batching_planner::UTKey;
use object_store::path::Path;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::NUM_CHUNKS;

pub(crate) const KEYS_QUERIES_PREFIX: &str = "V1_QUERIES";

type QueryId = String;

type RowKeyId = String;

type BlockNr = u64;

const ROWS_TREE: &str = "rows_tree";

const INDEX_TREE: &str = "index_tree";

const ROWS_CHUNK: &str = "rows_chunk";

const NON_EXISTENCE: &str = "non_existence";

const REVELATION: &str = "revelation";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
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

    RowsChunk(
        QueryId,
        UTKey<NUM_CHUNKS>,
    ),

    NonExistence(QueryId),

    Revelation(QueryId),
}

impl Display for ProofKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ProofKey::Row(query_id, block_nr, row_key_id) => {
                write!(
                    f,
                    "{KEYS_QUERIES_PREFIX}/{query_id}/{ROWS_TREE}/{block_nr}/{row_key_id}",
                )
            },
            ProofKey::Index(query_id, block_nr) => {
                write!(
                    f,
                    "{KEYS_QUERIES_PREFIX}/{query_id}/{INDEX_TREE}/{block_nr}",
                )
            },
            ProofKey::RowsChunk(query_id, key) => {
                // The `level` of the node in the `UpdateTree`.
                let level = key
                    .0
                     .0;
                // - The `position` of the node in the tree among the nodes with the same level.
                let position = key
                    .0
                     .1;
                write!(
                    f,
                    "{KEYS_QUERIES_PREFIX}/{query_id}/{ROWS_CHUNK}/{level}/{position}",
                )
            },
            ProofKey::NonExistence(query_id) => {
                write!(
                    f,
                    "{KEYS_QUERIES_PREFIX}/{query_id}/{NON_EXISTENCE}",
                )
            },
            ProofKey::Revelation(query_id) => {
                write!(
                    f,
                    "{KEYS_QUERIES_PREFIX}/{query_id}/{REVELATION}",
                )
            },
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
