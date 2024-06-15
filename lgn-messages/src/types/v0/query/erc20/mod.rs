use crate::types::v0::query::erc20::keys::BlockNr;
use crate::types::{HashOutput, Position};
use derive_debug_plus::Dbg;
use ethers::addressbook::Address;
use ethers::prelude::U256;
use serde_derive::{Deserialize, Serialize};

use crate::types::v0::query::{QueryBlockData, QueryStateData};

pub mod keys;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// Which contract this task is for.
    pub contract: Address,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    StorageEntry(StorageData),

    #[serde(rename = "2")]
    StateEntry(QueryStateData),

    #[serde(rename = "3")]
    BlocksDb(QueryBlockData),

    #[serde(rename = "4")]
    Revelation(RevelationData),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StorageData {
    StorageLeaf(StorageLeafInput),
    StorageBranch(StorageBranchInput),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum BlocksDbData {
    BlockPartialNode(BlockPartialNodeInput),
    BlockFullNode(BlockFullNodeInput),
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct StorageLeafInput {
    pub block_number: BlockNr,
    pub position: Position,
    pub query_address: Address,
    pub used_address: Address,
    pub value: U256,
    pub total_supply: U256,
    pub rewards_rate: U256,
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct StorageBranchInput {
    pub block_number: BlockNr,
    pub position: Position,
    pub left_child: Vec<u8>,
    pub right_child: Vec<u8>,
    pub proved_is_right: bool,
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct StateInput {
    pub smart_contract_address: Address,
    pub mapping_slot: u32,
    pub length_slot: u32,
    pub block_number: u64,
    pub proof: Option<Vec<(Position, HashOutput)>>,
    pub block_hash: HashOutput,
    pub storage_proof: Vec<u8>,
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct BlockPartialNodeInput {
    pub position: Position,

    pub child_position: Position,

    pub unproven_child_hash: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub child_proof: Vec<u8>,
    pub sibling_hash: HashOutput,
    pub sibling_is_left: bool,
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct BlockFullNodeInput {
    pub position: Position,
    pub left_proof: Vec<u8>,
    pub right_proof: Vec<u8>,
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

    /// the proof of the query tree
    pub erc2_proof_position: Position,

    #[dbg(skip)]
    pub erc2_proof: Vec<u8>,

    #[dbg(skip)]
    pub block_db_proof: Vec<u8>,
}
