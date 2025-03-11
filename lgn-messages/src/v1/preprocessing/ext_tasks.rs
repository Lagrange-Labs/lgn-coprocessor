use alloy_primitives::Address;
use derive_debug_plus::Dbg;
use ethers::types::H256;
use mp2_common::eth::node_type;
use mp2_common::eth::NodeType;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::BlockNr;
use crate::TableHash;

pub type Identifier = u64;
pub type MptNodeVersion = (BlockNr, H256);

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BatchedLength {
    pub table_hash: TableHash,
    pub block_nr: BlockNr,
    pub length_slot: usize,
    pub variable_slot: usize,

    #[dbg(placeholder = "...")]
    pub nodes: Vec<Vec<u8>>,
}

impl BatchedLength {
    pub fn extraction_types(&self) -> anyhow::Result<Vec<NodeType>> {
        self.nodes.iter().map(|node| node_type(node)).collect()
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchedContract {
    pub block_nr: BlockNr,
    pub storage_root: Vec<u8>,
    pub contract: Address,

    #[dbg(placeholder = "...")]
    pub nodes: Vec<Vec<u8>>,
}

impl BatchedContract {
    pub fn extraction_types(&self) -> anyhow::Result<Vec<NodeType>> {
        self.nodes.iter().map(|node| node_type(node)).collect()
    }
}
