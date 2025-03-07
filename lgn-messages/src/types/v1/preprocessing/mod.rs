use alloy_primitives::Address;
use alloy_primitives::U256;
use ethers::prelude::H256;
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

pub mod db_keys;
pub mod db_tasks;
pub mod ext_keys;
pub mod ext_tasks;

const KEYS_PREPROCESSING_PREFIX: &str = "V1_PREPROCESSING";

#[derive(Deserialize, Serialize)]
pub struct WorkerTask {
    /// Which block we are proving.
    pub block_nr: BlockNr,

    /// Chain ID
    pub chain_id: u64,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(
        chain_id: u64,
        block_nr: BlockNr,
        task_type: WorkerTaskType,
    ) -> Self {
        Self {
            chain_id,
            block_nr,
            task_type,
        }
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
        cells_proof_location: Option<db_keys::ProofKey>,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Leaf(RowLeafInput {
            table_id,
            row_id,
            identifier,
            value,
            is_multiplier,
            cells_proof_location,
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
        cells_proof_location: Option<db_keys::ProofKey>,
        child_proof_location: db_keys::ProofKey,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Partial(
            db_tasks::RowPartialInput {
                table_id,
                row_id,
                identifier,
                value,
                is_multiplier,
                is_child_left,
                child_proof_location,
                cells_proof_location,
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
        cells_proof_location: Option<db_keys::ProofKey>,
        child_proofs_locations: Vec<db_keys::ProofKey>,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Full(
            db_tasks::RowFullInput {
                table_id,
                row_id,
                identifier,
                value,
                is_multiplier,
                child_proofs_locations,
                cells_proof_location,
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
