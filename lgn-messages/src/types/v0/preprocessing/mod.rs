use std::collections::HashMap;
use std::fmt::Debug;

use derive_debug_plus::Dbg;
use ethers::types::Address;
use serde_derive::{Deserialize, Serialize};

use crate::types::{HashOutput, KeyedPayload, Position};

pub mod keys;

pub const ROUTING_DOMAIN: &str = "sp";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Which block we are proving.
    pub block_nr: u64,

    /// Chain ID
    pub chain_id: u64,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(chain_id: u64, block_nr: u64, task_type: WorkerTaskType) -> Self {
        Self {
            chain_id,
            block_nr,
            task_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Mpt(MptData),

    #[serde(rename = "2")]
    StorageDb(StorageDbData),

    #[serde(rename = "3")]
    LengthExtract(LengthExtractInput),

    #[serde(rename = "4")]
    LengthMatch(LengthMatchData),

    #[serde(rename = "5")]
    Equivalence(EquivalenceData),

    #[serde(rename = "6")]
    StateDb(StateDbData),

    #[serde(rename = "7")]
    BlockLinking(BlockLinkingInput),

    #[serde(rename = "8")]
    BlocksDb(BlocksDbData),
}

/// RLP encoded node
pub type RlpNode = Vec<u8>;

/// Merkle node
pub type MerkleNode = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MptData {
    Leaf(MptProofLeafData),
    Branch(MptProofBranchData),
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct MptProofLeafData {
    /// Contract address
    pub contract: Address,

    /// Storage slot of the mapping
    pub storage_slot: u8,

    /// Full key for a leaf inside this subtree
    pub mapping_key: Vec<u8>,

    /// MPT node
    pub node: Vec<u8>,

    /// MPT node hash
    pub hash: ethers::types::H256,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct MptProofBranchData {
    /// Contract address
    pub contract: Address,

    /// MPT node hash
    pub hash: ethers::types::H256,

    /// MPT branch node children, those that are part of partial MPT tree.
    pub children_copy_on_write_info: HashMap<ethers::types::H256, u64>,

    /// MPT node
    pub node: Vec<u8>,

    /// Recursive proofs
    #[dbg(skip)]
    pub child_proofs: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum StorageDbData {
    Leaf(StorageDbLeafData),
    Branch(StorageDbBranchData),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct StorageDbLeafData {
    /// Contract address
    pub contract: Address,

    /// Leaf position in storage tree
    pub position: Position,

    /// Mapping key
    pub key: Vec<u8>,

    /// Mapping value
    pub value: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct StorageDbBranchData {
    /// Contract address
    pub contract: Address,

    /// Branch node position in storage tree
    pub position: Position,

    /// Copy-on-Write info for children
    pub children_copy_on_write_info: Vec<(Position, u64)>,

    /// Left child proof in bytes
    #[dbg(skip)]
    pub left_child_proof: Vec<u8>,

    /// Right child proof in bytes
    #[dbg(skip)]
    pub right_child_proof: Vec<u8>,
}

impl StorageDbBranchData {
    #[must_use]
    pub fn children_positions(&self) -> Vec<Position> {
        let mut children = vec![];
        for i in 0..2 {
            let position = Position::new(self.position.level - 1, self.position.index * 2 + i);
            children.push(position);
        }
        children
    }

    #[must_use]
    pub fn children_copy_on_write_positions(&self) -> Vec<Position> {
        self.children_copy_on_write_info
            .iter()
            .map(|(pos, _)| *pos)
            .collect()
    }
}

#[derive(Clone, Dbg, PartialEq, Hash, Deserialize, Serialize)]
pub struct LengthExtractInput {
    /// Contract address
    pub contract: Address,

    /// Storage slot of the variable holding the length
    pub length_slot: u8,

    /// Proofs of the MPT node containing the length in the storage MPT trie
    #[dbg(skip)]
    pub mpt_nodes: Vec<Vec<u8>>,
}

/// Currently empty, we know where we have stored the [`WorkerTaskType::LengthExtract`] and [`WorkerTaskType::Mpt`] proofs.
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct LengthMatchData {
    /// Contract address
    pub contract: Address,

    /// Block number
    pub block_nr: u64,

    /// Location indicator
    pub mpt_root_hash: ethers::types::H256,

    /// Mapping entries proof in bytes
    #[dbg(skip)]
    pub mapping_proof: Vec<u8>,

    /// Length extract proof in bytes
    #[dbg(skip)]
    pub length_extract_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct EquivalenceData {
    /// Contract address
    pub contract: Address,

    /// Indicates where the storage root proof is stored
    pub storage_root_position: Position,

    pub storage_root_block_nr: u64,

    /// Length match proof in bytes
    #[dbg(skip)]
    pub length_match_proof: Vec<u8>,

    /// Storage proof in bytes
    #[dbg(skip)]
    pub storage_proof: Vec<u8>,
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct BlockLinkingInput {
    /// Contract address
    pub contract: Address,

    #[dbg(skip)]
    pub header_rlp: Vec<u8>,

    /// Account state proof in bytes
    #[dbg(skip)]
    pub account_proof: Vec<Vec<u8>>,

    /// When was the last time contract mapping was updated
    pub last_block_updated: u64,

    /// Equivalence proof in bytes
    #[dbg(skip)]
    pub equivalence_proof: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum StateDbData {
    Leaf(StateDbLeafData),
    Branch(StateDbBranchData),
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct StateDbLeafData {
    /// Leaf position in state tree
    pub position: Position,

    /// Block linking proof in bytes
    #[dbg(skip)]
    pub block_linking_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct StateDbBranchData {
    /// Branch node position in state tree
    pub position: Position,

    /// Left child proof in bytes
    #[dbg(skip)]
    pub left_proof: Vec<u8>,

    /// Right child proof in bytes
    #[dbg(skip)]
    pub right_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlocksDbData {
    /// Node index in the blocks database tree
    pub leaf_index: usize,

    /// Previous block hash
    pub previous_root_hash: HashOutput,

    /// Merkle proof of this node
    pub merkle_path: Vec<HashOutput>,

    /// Indicates the position of the state root in the state tree
    pub state_root_position: Position,

    /// New leaf proof in bytes
    #[dbg(skip)]
    pub new_leaf_proof: Vec<u8>,

    /// Previous leaf proof in bytes
    #[dbg(skip)]
    pub previous_leaf_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WorkerReply {
    pub chain_id: u64,
    pub block_nr: u64,
    #[dbg(formatter = crate::types::kp_pretty)]
    pub proof: Option<KeyedPayload>,
}

impl WorkerReply {
    #[must_use]
    pub fn new(chain_id: u64, block_nr: u64, proof: Option<KeyedPayload>) -> Self {
        Self {
            chain_id,
            block_nr,
            proof,
        }
    }
}
