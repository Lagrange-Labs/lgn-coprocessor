use crate::types::v0::{PARAMS_VERSION, STORAGE_QUERY2};
use ethers::abi::Address;
use std::fmt::Display;

use crate::types::Position;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};

pub type QueryId = String;

pub type BlockNr = u64;

/// Assume that we only have single mapping per contract that we are proving(at least for now)
pub type Contract = Address;

pub type Key = String;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    PublicParams,

    /// Where to store proof of storage computation for a single mapping entry
    StorageEntry(QueryId, BlockNr, Contract, Position),

    /// Where to store proof of query for state database
    StateDatabase(QueryId, BlockNr, Contract),

    /// Where to store aggregation proofs
    Aggregation(QueryId, Position),

    /// Where to store proof of aggregation
    Revelation(QueryId),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum QueryInfo {
    Result(String),
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::PublicParams => {
                // Example: latest/STORAGE_QUERY2/public_params
                write!(f, "{PARAMS_VERSION}/{STORAGE_QUERY2}/public_params")
            }
            ProofKey::StorageEntry(query_id, block_nr, contract, position) => {
                let level = position.level;
                let index = position.index;
                write!(
                    f,
                    "{STORAGE_QUERY2}/{query_id}/{contract}/{block_nr}/storage/{level}/{index}"
                )
            }
            ProofKey::StateDatabase(query_id, block_nr, contract) => {
                write!(f, "{STORAGE_QUERY2}/{query_id}/{contract}/{block_nr}/state")
            }
            ProofKey::Aggregation(query_id, position) => {
                let level = position.level;
                let index = position.index;
                write!(f, "{STORAGE_QUERY2}/{query_id}/block/{level}/{index}")
            }
            ProofKey::Revelation(query_id) => {
                write!(f, "{STORAGE_QUERY2}/{query_id}/revelation")
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
    fn from(key: ProofKey) -> String {
        key.to_string()
    }
}

impl Display for QueryInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryInfo::Result(query_id) => {
                write!(f, "{STORAGE_QUERY2}/query_info/{query_id}")
            }
        }
    }
}

impl From<QueryInfo> for Path {
    fn from(key: QueryInfo) -> Self {
        Path::from(key.to_string())
    }
}
