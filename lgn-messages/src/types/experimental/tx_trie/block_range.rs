use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::experimental::tx_trie::keys::ProofKey;

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ProofKind {
    /// Proves nodes in blocks tree
    #[serde(rename = "1")]
    Blocks(Blocks),
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct Blocks {
    pub data_uris: Vec<ProofKey>,
}

impl Blocks {
    #[must_use]
    pub fn new(data_uris: Vec<ProofKey>) -> Self {
        Self {
            data_uris,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct Reply {
    /// Indicates where proof was stored
    pub data_uri: String,
}
