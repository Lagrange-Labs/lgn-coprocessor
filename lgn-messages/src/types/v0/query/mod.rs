pub mod keys;

use crate::types::{HashOutput, KeyedPayload, Position};
use derive_debug_plus::Dbg;
use ethers::types::Address;
use serde_derive::{Deserialize, Serialize};

pub const ROUTING_DOMAIN: &str = "sc";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,
    /// Query ID
    pub query_id: String,

    /// Which contract this task is for.
    pub contract: Address,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(
        chain_id: u64,
        query_id: String,
        contract: Address,
        task_type: WorkerTaskType,
    ) -> Self {
        Self {
            chain_id,
            query_id,
            contract,
            task_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    StorageEntry(StorageData),

    #[serde(rename = "2")]
    StateEntry(Query2StateData),

    #[serde(rename = "3")]
    Aggregation(Query2BlockData),

    #[serde(rename = "4")]
    Revelation(RevelationData),
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct StorageData {
    /// Which block
    pub block_nr: u64,
    pub position: Position,
    pub details: StorageDetails,
    #[dbg(skip)]
    pub inputs: StorageProofInput,
}

#[derive(Clone, Dbg, Serialize, Deserialize)]
pub enum StorageProofInput {
    Leaf {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    FullBranch {
        #[dbg(skip)]
        left_child_proof: Vec<u8>,
        #[dbg(skip)]
        right_child_proof: Vec<u8>,
    },
    PartialBranch {
        #[dbg(skip)]
        proven_child: Vec<u8>,
        unproven_child_hash: Vec<u8>,
        right_is_proven: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum StorageDetails {
    Leaf {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    FullBranch,
    PartialBranch {
        proven_child_position: Position,
        unproven_child_hash: Vec<u8>,
    },
}

#[derive(Clone, Serialize, Deserialize, Dbg, PartialEq, Default)]
pub struct Query2StateData {
    /// smart contract over which we're proving
    pub smart_contract_address: Address,

    /// corresponding mapping slot
    pub mapping_slot: u32,

    /// corresponding length slot
    pub length_slot: u32,

    /// block number to which we prove the state belongs to
    pub block_number: u64,

    /// corresponding block hash associated
    pub block_hash: HashOutput,

    /// Root of the LPN storage db
    pub state_root: HashOutput,

    #[dbg(skip)]
    pub proof: Option<Vec<(Position, HashOutput)>>,

    pub storage_root_position: Position,

    #[dbg(skip)]
    pub storage_proof: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct StateLeafData {
    pub position: Position,
    pub block_nr: u64,
    pub sibling_hash: Option<HashOutput>,
}

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct StateBranchData {
    pub position: Position,
    pub block_nr: u64,
    pub hash: HashOutput,
    pub sibling_hash: Option<HashOutput>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Query2BlockData {
    FullNode(FullNodeBlockData),
    PartialNode(PartialNodeBlockData),
}

impl Query2BlockData {
    #[must_use]
    pub fn position(&self) -> Position {
        match self {
            Query2BlockData::FullNode(data) => data.position,
            Query2BlockData::PartialNode(data) => data.position,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash)]
pub enum BlockNodeLeafInfo {
    StateProof(u64),
    Aggregation(Position),
}

#[derive(Clone, Dbg, Serialize, Deserialize, PartialEq)]
pub struct FullNodeBlockData {
    /// Node position in blockchain database tree
    pub position: Position,

    /// Information about the left child in blockchain database tree
    pub left_child_info: BlockNodeLeafInfo,

    /// Information about the right child in blockchain database tree
    pub right_child_info: BlockNodeLeafInfo,

    /// Left child proof in bytes
    #[dbg(skip)]
    pub left_child_proof: Vec<u8>,

    /// Right child proof in bytes
    #[dbg(skip)]
    pub right_child_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Serialize, Deserialize)]
pub struct PartialNodeBlockData {
    /// position, related to executor info
    pub position: Position,

    pub child_info: BlockNodeLeafInfo,

    /// hash of the child we don't have a proof for,i.e. the sibling
    pub sibling_hash: HashOutput,

    pub sibling_position: Position,

    #[dbg(skip)]
    pub child_proof: Vec<u8>,
}

#[derive(Clone, Dbg, Default, PartialEq, Deserialize, Serialize)]
pub struct RevelationData {
    /// NFT IDs being queried
    pub mapping_keys: Vec<Vec<u8>>,

    /// start of the queried block range
    pub query_min_block: usize,

    /// end of the queried block range
    pub query_max_block: usize,

    /// the proof of the query tree
    pub query2_proof_position: Position,

    /// the proof from the block databse
    pub block_db_proof_block_nr: u64,

    pub block_db_proof_block_leaf_index: usize,

    #[dbg(skip)]
    pub query2_proof: Vec<u8>,
    #[dbg(skip)]
    pub block_db_proof: Vec<u8>,
}

#[derive(Clone, Dbg, PartialEq, Eq, Deserialize, Serialize)]
pub struct WorkerReply {
    pub chain_id: u64,
    pub query_id: String,
    pub task_id: String,
    #[dbg(formatter = crate::types::kp_pretty)]
    pub proof: Option<KeyedPayload>,
}

impl WorkerReply {
    #[must_use]
    pub fn new(
        chain_id: u64,
        query_id: String,
        task_id: String,
        proof: Option<KeyedPayload>,
    ) -> Self {
        Self {
            chain_id,
            query_id,
            task_id,
            proof,
        }
    }
}
