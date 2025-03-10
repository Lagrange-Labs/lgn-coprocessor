use alloy_primitives::Address;
use alloy_primitives::U256;
use anyhow::bail;
use ethers::prelude::H256;
use ethers::utils::rlp::Rlp;
use mp2_common::digest::TableDimension;
use mp2_v1::values_extraction;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::preprocessing::db_tasks::DatabaseType;
use crate::types::v1::preprocessing::db_tasks::IvcInput;
use crate::types::v1::preprocessing::db_tasks::RowLeafInput;
use crate::types::v1::preprocessing::ext_tasks::BlockExtractionInput;
use crate::types::v1::preprocessing::ext_tasks::Contract;
use crate::types::v1::preprocessing::ext_tasks::ExtractionType;
use crate::types::v1::preprocessing::ext_tasks::FinalExtraction;
use crate::types::v1::preprocessing::ext_tasks::Identifier;
use crate::types::v1::preprocessing::ext_tasks::Length;
use crate::types::v1::preprocessing::ext_tasks::Mpt;
use crate::types::v1::preprocessing::ext_tasks::MptNodeVersion;
use crate::BlockNr;
use crate::TableHash;
use crate::TableId;

pub mod db_tasks;
pub mod ext_tasks;

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
pub struct WorkerTask {
    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(task_type: WorkerTaskType) -> Self {
        Self { task_type }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Extraction(ExtractionType),

    #[serde(rename = "2")]
    Database(DatabaseType),
}

impl WorkerTaskType {
    pub fn ext_variable(
        table_hash: TableHash,
        block_nr: BlockNr,
        node_hash: H256,
        circuit_input: values_extraction::CircuitInput,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::MptExtraction(Mpt {
            table_hash,
            block_nr,
            node_hash,
            circuit_input,
        }))
    }

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

    pub fn ext_block(rlp_header: Vec<u8>) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::BlockExtraction(BlockExtractionInput::new(
            rlp_header,
        )))
    }

    pub fn ext_final_extraction_simple(
        table_id: TableId,
        table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        compound: TableDimension,
        value_proof_version: MptNodeVersion,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::FinalExtraction(Box::new(
            FinalExtraction::new_single_table(
                table_id,
                table_hash,
                block_nr,
                contract,
                Some(compound),
                value_proof_version,
            ),
        )))
    }

    pub fn ext_final_extraction_lengthed(
        table_id: TableId,
        table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::FinalExtraction(Box::new(
            FinalExtraction::new_single_table(
                table_id,
                table_hash,
                block_nr,
                contract,
                None,
                value_proof_version,
            ),
        )))
    }

    pub fn ext_final_extraction_merge(
        table_id: TableId,
        simple_table_hash: TableHash,
        mapping_table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::FinalExtraction(Box::new(
            FinalExtraction::new_merge_table(
                table_id,
                simple_table_hash,
                mapping_table_hash,
                block_nr,
                contract,
                value_proof_version,
            ),
        )))
    }

    pub fn db_cells_tree(
        table_id: TableId,
        row_id: String,
        cell_id: usize,
        circuit_input: verifiable_db::cells_tree::CircuitInput,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Cell {
            table_id,
            row_id,
            cell_id,
            circuit_input,
        })
    }

    pub fn db_row_leaf(
        table_id: TableId,
        row_id: String,
        identifier: Identifier,
        value: U256,
        is_multiplier: bool,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Leaf(RowLeafInput {
            table_id,
            row_id,
            identifier,
            value,
            is_multiplier,
            cells_proof: vec![],
        })))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn db_row_partial(
        table_id: TableId,
        row_id: String,
        identifier: Identifier,
        value: U256,
        is_multiplier: bool,
        is_child_left: bool,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Partial(
            db_tasks::RowPartialInput {
                table_id,
                row_id,
                identifier,
                value,
                is_multiplier,
                is_child_left,
                child_proof: vec![],
                cells_proof: vec![],
            },
        )))
    }

    pub fn db_row_full(
        table_id: TableId,
        row_id: String,
        identifier: Identifier,
        value: U256,
        is_multiplier: bool,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Full(
            db_tasks::RowFullInput {
                table_id,
                row_id,
                identifier,
                value,
                is_multiplier,
                child_proofs: vec![],
                cells_proof: vec![],
            },
        )))
    }

    pub fn ivc(
        table_id: TableId,
        block_nr: BlockNr,
        is_first_block: bool,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::IVC(IvcInput::new(
            table_id,
            block_nr,
            is_first_block,
        )))
    }
}
