use alloy_primitives::Address;
use anyhow::bail;
use ethers::utils::rlp::Rlp;
use mp2_v1::api::MAX_FIELD_PER_EVM;
use mp2_v1::values_extraction;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::query::MAX_NUM_COLUMNS;
use super::ConcreteCircuitInput;
use crate::types::v1::preprocessing::db_tasks::DatabaseType;
use crate::types::v1::preprocessing::ext_tasks::Contract;
use crate::types::v1::preprocessing::ext_tasks::ExtractionType;
use crate::types::v1::preprocessing::ext_tasks::Length;
use crate::BlockNr;
use crate::TableHash;

pub mod db_tasks;
pub mod ext_tasks;

pub type ConcreteValueExtractionCircuitInput =
    values_extraction::CircuitInput<69, MAX_NUM_COLUMNS, MAX_FIELD_PER_EVM>;

/// Different types of node types.
#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Branch,
    Extension,
    Leaf,
}

/// Returns the node type given an encoded node.
///
/// The node spec is at [1].
///
/// 1- https://github.com/ethereum/execution-specs/blob/78fb726158c69d8fa164e28f195fabf6ab59b915/src/ethereum/cancun/trie.py#L177-L191
pub fn node_type(rlp_data: &[u8]) -> anyhow::Result<NodeType> {
    let rlp = Rlp::new(rlp_data);

    let item_count = rlp.item_count()?;

    if item_count == 17 {
        Ok(NodeType::Branch)
    } else if item_count == 2 {
        // The first item is the encoded path, if it begins with a 2 or 3 it is a leaf, else it is
        // an extension node
        let first_item = rlp.at(0)?;

        // We want the first byte
        let first_byte = first_item.as_raw()[0];

        // The we divide by 16 to get the first nibble
        match first_byte / 16 {
            0 | 1 => Ok(NodeType::Extension),
            2 | 3 => Ok(NodeType::Leaf),
            _ => {
                bail!("Expected compact encoding beginning with 0,1,2 or 3")
            },
        }
    } else {
        bail!("RLP encoded Node item count was {item_count}, expected either 17 or 2")
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Extraction(ExtractionType),

    #[serde(rename = "2")]
    Database(DatabaseType),

    #[serde(rename = "3")]
    CircuitInput(ConcreteCircuitInput),
}

impl WorkerTaskType {
    pub fn ext_length(
        table_hash: TableHash,
        block_nr: BlockNr,
        nodes: Vec<Vec<u8>>,
        length_slot: usize,
        variable_slot: usize,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::LengthExtraction(Length {
            table_hash,
            block_nr,
            length_slot,
            variable_slot,
            nodes,
        }))
    }

    pub fn ext_contract(
        block_nr: BlockNr,
        contract_address: Address,
        nodes: Vec<Vec<u8>>,
        storage_root: Vec<u8>,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::ContractExtraction(Contract {
            block_nr,
            storage_root,
            contract: contract_address,
            nodes,
        }))
    }
}
