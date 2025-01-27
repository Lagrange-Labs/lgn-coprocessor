use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::experimental::tx_trie::keys::ProofKey;

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ProofKind {
    /// Proof of transactions(leaves) in a block transaction trie
    #[serde(rename = "1")]
    Transactions(Transactions),

    /// Proof of intermediate node in a block transaction trie
    #[serde(rename = "2")]
    Intermediate(Intermediate),
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct Transactions {
    /// Which block we are proving
    pub block_nr: u64,

    /// Start of the range of transactions in the block transaction trie
    pub tx_start: u64,

    /// End of the range of transactions in the block transaction trie
    pub tx_end: u64,
}

impl Transactions {
    #[must_use]
    pub fn new(
        block_nr: u64,
        tx_start: u64,
        tx_end: u64,
    ) -> Self {
        Self {
            block_nr,
            tx_start,
            tx_end,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct Intermediate {
    /// Which block we are proving
    pub block_nr: u64,

    /// Which intermediate node we prove in the trie
    pub node_id: String,

    /// Where to receive child nodes proofs from
    pub data_uris: Vec<ProofKey>,
}

impl Intermediate {
    #[must_use]
    pub fn new(
        block_nr: u64,
        node_id: String,
        data_uris: Vec<ProofKey>,
    ) -> Self {
        Self {
            block_nr,
            node_id,
            data_uris,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct Reply {
    /// Indicates where proof was stored
    pub data_uri: String,
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct IntermediaryResultReply {
    /// Indicates where proof was stored
    pub data_uri: String,
}
