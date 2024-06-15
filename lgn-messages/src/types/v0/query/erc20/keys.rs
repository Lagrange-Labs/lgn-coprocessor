use crate::types::Position;
use ethers::abi::Address;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

pub type QueryId = String;

pub type BlockNr = u64;

/// Assume that we only have single mapping per contract that we are proving(at least for now)
pub type Contract = Address;

pub type Key = String;

pub(crate) const ERC20_QUERY: &str = "ERC20_QUERY";

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Where to store proof of storage computation for a single mapping entry
    StorageEntry(QueryId, BlockNr, Position),

    /// Where to store proof of query for state database
    StateDatabase(QueryId, BlockNr),

    /// Where to store aggregation proofs
    Aggregation(QueryId, Position),

    /// Where to store proof of aggregation
    Revelation(QueryId),
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::StorageEntry(query_id, block_nr, position) => {
                let level = position.level;
                let index = position.index;
                write!(
                    f,
                    "{ERC20_QUERY}/{query_id}/{block_nr}/storage/{level}/{index}"
                )
            }
            ProofKey::StateDatabase(query_id, block_nr) => {
                write!(f, "{ERC20_QUERY}/{query_id}/{block_nr}/state")
            }
            ProofKey::Aggregation(query_id, position) => {
                let level = position.level;
                let index = position.index;
                write!(f, "{ERC20_QUERY}/{query_id}/block/{level}/{index}")
            }
            ProofKey::Revelation(query_id) => {
                write!(f, "{ERC20_QUERY}/{query_id}/revelation")
            }
        }
    }
}
