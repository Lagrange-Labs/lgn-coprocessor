use alloy_primitives::{Address, U256};
use ethers::prelude::H256;
use serde_derive::{Deserialize, Serialize};

use crate::types::v1::preprocessing::db_tasks::{
    CellFullInput, CellLeafInput, CellPartialInput, DatabaseType, IvcInput, RowLeafInput,
};
use crate::types::v1::preprocessing::ext_tasks::{
    BlockExtractionInput, Contract, ExtractionType, FinalExtraction, Identifier, Length,
    MappingBranchInput, MappingLeafInput, Mpt, MptNodeVersion, MptType, VariableBranchInput,
    VariableLeafInput,
};

pub mod db_keys;
pub mod db_tasks;
pub mod ext_keys;
pub mod ext_tasks;

const KEYS_PREPROCESSING_PREFIX: &str = "V1_PREPROCESSING";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Extraction(ExtractionType),

    #[serde(rename = "2")]
    Database(DatabaseType),
}

impl WorkerTaskType {
    pub fn ext_variable_leaf(
        table_id: u64,
        block_nr: u64,
        node_hash: H256,
        key: Vec<u8>,
        node: Vec<u8>,
        slot: usize,
        contract_address: Address,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::MptExtraction(Mpt {
            table_id,
            block_nr,
            node_hash,
            mpt_type: MptType::VariableLeaf(VariableLeafInput::new(
                table_id,
                key,
                node,
                slot,
                contract_address,
            )),
        }))
    }

    pub fn ext_variable_branch(
        table_id: u64,
        block_nr: u64,
        node_hash: H256,
        node: Vec<u8>,
        children: Vec<MptNodeVersion>,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::MptExtraction(Mpt {
            table_id,
            block_nr,
            node_hash,
            mpt_type: MptType::VariableBranch(VariableBranchInput::new(table_id, node, children)),
        }))
    }
    pub fn ext_mapping_leaf(
        table_id: u64,
        block_nr: u64,
        node_hash: H256,
        key: Vec<u8>,
        node: Vec<u8>,
        slot: usize,
        contract_address: Address,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::MptExtraction(Mpt {
            table_id,
            block_nr,
            node_hash,
            mpt_type: MptType::MappingLeaf(MappingLeafInput::new(
                key,
                node,
                slot,
                contract_address,
            )),
        }))
    }

    pub fn ext_mapping_branch(
        table_id: u64,
        block_nr: u64,
        node_hash: H256,
        node: Vec<u8>,
        children: Vec<MptNodeVersion>,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::MptExtraction(Mpt {
            table_id,
            block_nr,
            node_hash,
            mpt_type: MptType::MappingBranch(MappingBranchInput::new(node, children)),
        }))
    }

    pub fn ext_length(
        table_id: u64,
        block_nr: u64,
        nodes: Vec<Vec<u8>>,
        length_slot: usize,
        variable_slot: usize,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::LengthExtraction(Length {
            table_id,
            block_nr,
            length_slot,
            variable_slot,
            nodes,
        }))
    }

    pub fn ext_contract(
        block_nr: u64,
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
        table_id: u64,
        block_nr: u64,
        contract: Address,
        compound: bool,
        value_proof_version: MptNodeVersion,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::FinalExtraction(FinalExtraction::new(
            table_id,
            block_nr,
            contract,
            Some(compound),
            value_proof_version,
        )))
    }

    pub fn ext_final_extraction_lengthed(
        table_id: u64,
        block_nr: u64,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> WorkerTaskType {
        WorkerTaskType::Extraction(ExtractionType::FinalExtraction(FinalExtraction::new(
            table_id,
            block_nr,
            contract,
            None,
            value_proof_version,
        )))
    }

    pub fn db_cell_leaf(
        table_id: u64,
        row_id: String,
        cell_id: usize,
        identifier: Identifier,
        value: U256,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Cell(db_tasks::DbCellType::Leaf(
            CellLeafInput {
                table_id,
                row_id,
                cell_id,
                identifier,
                value,
            },
        )))
    }

    pub fn db_cell_partial(
        table_id: u64,
        row_id: String,
        cell_id: usize,
        identifier: Identifier,
        value: U256,
        child_location: db_keys::ProofKey,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Cell(db_tasks::DbCellType::Partial(
            CellPartialInput {
                table_id,
                row_id,
                cell_id,
                identifier,
                value,
                child_location,
                child_proof: vec![],
            },
        )))
    }

    pub fn db_cell_full(
        table_id: u64,
        row_id: String,
        cell_id: usize,
        identifier: Identifier,
        value: U256,
        child_locations: Vec<db_keys::ProofKey>,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Cell(db_tasks::DbCellType::Full(
            CellFullInput {
                table_id,
                row_id,
                cell_id,
                identifier,
                value,
                child_locations,
                child_proofs: vec![],
            },
        )))
    }

    pub fn db_row_leaf(
        table_id: u64,
        row_id: String,
        identifier: Identifier,
        value: U256,
        cells_proof_location: Option<db_keys::ProofKey>,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Leaf(RowLeafInput {
            table_id,
            row_id,
            identifier,
            value,
            cells_proof_location,
            cells_proof: vec![],
        })))
    }

    pub fn db_row_partial(
        table_id: u64,
        row_id: String,
        identifier: Identifier,
        value: U256,
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
                is_child_left,
                child_proof_location,
                cells_proof_location,
                child_proof: vec![],
                cells_proof: vec![],
            },
        )))
    }

    pub fn db_row_full(
        table_id: u64,
        row_id: String,
        identifier: Identifier,
        value: U256,
        cells_proof_location: Option<db_keys::ProofKey>,
        child_proofs_locations: Vec<db_keys::ProofKey>,
    ) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::Row(db_tasks::DbRowType::Full(
            db_tasks::RowFullInput {
                table_id,
                row_id,
                identifier,
                value,
                child_proofs_locations,
                cells_proof_location,
                child_proofs: vec![],
                cells_proof: vec![],
            },
        )))
    }

    pub fn ivc(table_id: u64, block_nr: u64, is_first_block: bool) -> WorkerTaskType {
        WorkerTaskType::Database(DatabaseType::IVC(IvcInput::new(
            table_id,
            block_nr,
            is_first_block,
        )))
    }
}
