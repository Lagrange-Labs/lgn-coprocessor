use ethers::prelude::{Address, H256};
use serde_derive::{Deserialize, Serialize};

pub const ROUTING_DOMAIN: &str = "sp";

pub type MptNodeVersion = (u64, H256);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DetailsLater;

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
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ExtractionType {
    #[serde(rename = "1")]
    MptExtraction(Mpt),

    #[serde(rename = "2")]
    LengthExtraction(Length),

    #[serde(rename = "3")]
    ContractExtraction(Contract),

    #[serde(rename = "4")]
    BlockExtraction(BlockExtractionInput),

    #[serde(rename = "5")]
    FinalExtraction(FinalExtraction),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Mpt {
    pub table_id: u64,
    pub block_nr: u64,
    pub node_hash: H256,
    pub mpt_type: MptType,
}

impl Mpt {
    pub fn new(table_id: u64, block_nr: u64, node_hash: H256, mpt_type: MptType) -> Self {
        Self {
            table_id,
            block_nr,
            node_hash,
            mpt_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum MptType {
    #[serde(rename = "1")]
    MappingLeaf(MappingLeafInput),

    #[serde(rename = "2")]
    MappingBranch(MappingBranchInput),

    #[serde(rename = "3")]
    VariableLeaf(VariableLeafInput),

    #[serde(rename = "4")]
    VariableBranch(VariableBranchInput),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum DatabaseType {
    #[serde(rename = "1")]
    Cell(DetailsLater),

    #[serde(rename = "2")]
    Row(DetailsLater),

    #[serde(rename = "3")]
    Block(DetailsLater),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingLeafInput {
    pub key: Vec<u8>,
    pub node: Vec<u8>,
    pub slot: usize,
    pub contract_address: Address,
}

impl MappingLeafInput {
    pub fn new(key: Vec<u8>, node: Vec<u8>, slot: usize, contract_address: Address) -> Self {
        Self {
            key,
            node,
            slot,
            contract_address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingBranchInput {
    pub node: Vec<u8>,
    pub children: Vec<MptNodeVersion>,
    pub children_proofs: Vec<Vec<u8>>,
}

impl MappingBranchInput {
    pub fn new(node: Vec<u8>, children: Vec<MptNodeVersion>) -> Self {
        Self {
            node,
            children,
            children_proofs: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableLeafInput {
    pub table_id: u64,
    pub key: Vec<u8>,
    pub node: Vec<u8>,
    pub slot: usize,
    pub contract_address: Address,
}

impl VariableLeafInput {
    pub fn new(
        table_id: u64,
        key: Vec<u8>,
        node: Vec<u8>,
        slot: usize,
        contract_address: Address,
    ) -> Self {
        Self {
            table_id,
            key,
            node,
            slot,
            contract_address,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableBranchInput {
    pub table_id: u64,
    pub node: Vec<u8>,
    pub children: Vec<MptNodeVersion>,
    pub children_proofs: Vec<Vec<u8>>,
}

impl VariableBranchInput {
    pub fn new(table_id: u64, node: Vec<u8>, children: Vec<MptNodeVersion>) -> Self {
        Self {
            table_id,
            node,
            children,
            children_proofs: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Length {
    pub table_id: u64,
    pub block_nr: u64,
    pub length_slot: usize,
    pub variable_slot: usize,
    pub nodes: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub block_nr: u64,
    pub storage_root: Vec<u8>,
    pub contract: Address,
    pub nodes: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct BlockExtractionInput {
    pub rlp_header: Vec<u8>,
}

impl BlockExtractionInput {
    pub fn new(rlp_header: Vec<u8>) -> Self {
        Self { rlp_header }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct FinalExtraction {
    pub table_id: u64,

    pub block_nr: u64,

    pub contract: Address,

    /// This is always versioned because we prove values only if they changed.
    pub value_proof_version: MptNodeVersion,

    pub extraction_type: FinalExtractionType,

    pub block_proof: Vec<u8>,
    pub contract_proof: Vec<u8>,
    pub value_proof: Vec<u8>,
    pub length_proof: Vec<u8>,
}

impl FinalExtraction {
    pub fn new(
        table_id: u64,
        block_nr: u64,
        contract: Address,
        compound: Option<bool>,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        let extraction_type = match compound {
            Some(compound) => FinalExtractionType::Simple(compound),
            None => FinalExtractionType::Lengthed,
        };

        Self {
            table_id,
            block_nr,
            contract,
            value_proof_version,
            extraction_type,
            block_proof: vec![],
            contract_proof: vec![],
            value_proof: vec![],
            length_proof: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum FinalExtractionType {
    Simple(bool),
    Lengthed,
}
