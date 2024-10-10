use crate::types::v0::{PARAMS_VERSION, STORAGE_QUERY2};
use std::fmt::Display;

use ethers::abi::Address;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};

use crate::types::Position;

pub type BlockNr = u64;
pub type ChainId = u64;
pub type Contract = Address;
pub type MptNodeHash = String;
pub type QueryId = String;
pub type TableId = u64;

/// Level in binary tree
pub type Level = usize;

/// Index in binary tree in a level
pub type Index = usize;

#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Indicates the location of `PublicParams`.
    PublicParams,

    /// Indicates the location of `MPT` proof tree node.
    MptInclusion(BlockNr, Contract, MptNodeHash),

    /// Indicates the location of `Length slot` proof. Don't need more details because we only have one slot per contract.
    LengthSlot(BlockNr, Contract),

    /// Indicates the location of `StorageDb` proof tree nodes.
    StorageDb(BlockNr, Contract, Position),

    /// Indicates the location of `Bridge` proof. Don't need more details because we only have one bridge proof per contract.
    Bridge(BlockNr, Contract),

    /// Indicates the location of `Equivalence` proof tree nodes. Don't need more details because we only have one equivalence proof per contract.
    Equivalence(BlockNr, Contract),

    /// Indicates the location of `BlockHeader` proof tree nodes. Don't need more details because we only have one block header proof per contract.
    BlockLinking(BlockNr, Contract),

    /// Indicates the location of `State` proof tree nodes. Don't need more details because we only have one state proof per contract.
    State(BlockNr, Position),

    /// Indicates the location of `BlockDatabase` proof tree nodes. Don't need more details because we only have one block database proof per contract.
    BlocksDb(BlockNr, Index),
}

/// Computation databases
const STORAGE_DB: &str = "STORAGE_DB";
const STATE_DB: &str = "STATE_DB";
const BLOCKS_DB: &str = "BLOCKS_DB";

/// Preprocessing "databases"
const STORAGE_PREPROCESS: &str = "STORAGE_PREPROCESS";
const MPT: &str = "MPT";
const LENGTH_SLOT: &str = "LENGTH_SLOT";
const BLOCK_LINKING: &str = "BLOCK_LINKING";
const BRIDGE: &str = "BRIDGE";
const EQUIVALENCE: &str = "EQUIVALENCE";
const PUBLIC_PARAMS: &str = "PUBLIC_PARAMS";

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::PublicParams => {
                // Example: latest/STORAGE_PREPROCESS/PUBLIC_PARAMS
                write!(f, "{PARAMS_VERSION}/{STORAGE_PREPROCESS}/{PUBLIC_PARAMS}")
            }
            ProofKey::MptInclusion(block_nr, contract, mpt_node_hash) => {
                // Example: STORAGE_PREPROCESS/0xabcd/123/MPT/0x456
                write!(
                    f,
                    "{STORAGE_PREPROCESS}/{contract}/{block_nr}/{MPT}/{mpt_node_hash}"
                )
            }

            ProofKey::LengthSlot(block_nr, contract) => {
                // Example: STORAGE_PREPROCESS/0xabcd/123/LENGTH_SLOT
                write!(
                    f,
                    "{STORAGE_PREPROCESS}/{contract}/{block_nr}/{LENGTH_SLOT}"
                )
            }
            ProofKey::Bridge(block_nr, contract) => {
                // Example: STORAGE_PREPROCESS/0xabcd/123/BRIDGE
                write!(f, "{STORAGE_PREPROCESS}/{contract}/{block_nr}/{BRIDGE}")
            }
            ProofKey::Equivalence(block_nr, contract) => {
                // Example: STORAGE_PREPROCESS/0xabcd/123/EQUIVALENCE
                write!(
                    f,
                    "{STORAGE_PREPROCESS}/{contract}/{block_nr}/{EQUIVALENCE}"
                )
            }
            ProofKey::BlockLinking(block_nr, contract) => {
                // Example: STORAGE_PREPROCESS/0xabcd/123/BLOCK_LINKING
                write!(
                    f,
                    "{STORAGE_PREPROCESS}/{contract}/{block_nr}/{BLOCK_LINKING}"
                )
            }

            ProofKey::StorageDb(block_nr, contract, position) => {
                // Example: STORAGE_QUERY2/0xabcd/123/STORAGE_DB/0/0
                let level = position.level;
                let index = position.index;
                write!(
                    f,
                    "{STORAGE_QUERY2}/{contract}/{block_nr}/{STORAGE_DB}/{level}/{index}"
                )
            }

            ProofKey::State(block_nr, position) => {
                // Example: STORAGE_QUERY2/123/STATE_DB/0/0
                let level = position.level;
                let index = position.index;
                write!(f, "{STORAGE_QUERY2}/{block_nr}/{STATE_DB}/{level}/{index}")
            }

            ProofKey::BlocksDb(block_nr, index) => {
                // Example: STORAGE_QUERY2/123/BLOCKS_DB/0
                write!(f, "{STORAGE_QUERY2}/{block_nr}/{BLOCKS_DB}/{index}")
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
