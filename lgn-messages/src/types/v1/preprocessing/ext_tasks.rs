use crate::types::v1::preprocessing::ext_keys::ProofKey;
use crate::types::v1::preprocessing::{WorkerTask, WorkerTaskType};
use alloy_primitives::Address;
use derive_debug_plus::Dbg;
use ethers::{types::H256, utils::rlp};
use mp2_v1::values_extraction::{
    identifier_for_mapping_key_column, identifier_for_mapping_value_column,
    identifier_single_var_column,
};
use serde_derive::{Deserialize, Serialize};

pub const ROUTING_DOMAIN: &str = "sp";

pub type Identifier = u64;

pub type MptNodeVersion = (u64, H256);

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingLeafInput {
    pub key: Vec<u8>,
    pub node: Vec<u8>,
    pub slot: u8,
    pub key_id: u64,
    pub value_id: u64,
}

impl MappingLeafInput {
    pub fn new(
        key: Vec<u8>,
        node: Vec<u8>,
        slot: u8,
        contract_address: &Address,
        chain_id: u64,
    ) -> Self {
        let key_id = identifier_for_mapping_key_column(slot, contract_address, chain_id, vec![]);
        let value_id =
            identifier_for_mapping_value_column(slot, contract_address, chain_id, vec![]);

        Self {
            key,
            node,
            slot,
            key_id,
            value_id,
        }
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingBranchInput {
    pub node: Vec<u8>,

    pub children: Vec<MptNodeVersion>,

    #[dbg(placeholder = "...")]
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
    pub node: Vec<u8>,
    pub slot: u8,
    pub column_id: u64,
}

impl VariableLeafInput {
    pub fn new(node: Vec<u8>, slot: u8, contract_address: &Address, chain_id: u64) -> Self {
        let column_id = identifier_single_var_column(slot, contract_address, chain_id, vec![]);
        Self {
            node,
            slot,
            column_id,
        }
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableBranchInput {
    pub table_id: u64,
    pub node: Vec<u8>,
    pub children: Vec<MptNodeVersion>,

    #[dbg(placeholder = "...")]
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

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct Length {
    pub table_id: u64,
    pub block_nr: u64,
    pub length_slot: usize,
    pub variable_slot: usize,

    #[dbg(placeholder = "...")]
    pub nodes: Vec<Vec<u8>>,
}

/// Helper type denoting if a proof request is for a leaf or extension or branch node.
///
/// This is used to associate the gas / time to the proof to generate, especially for tasks where
/// there are many proofs to generate inside.
pub enum MPTExtractionType {
    Branch,
    Extension,
    Leaf,
}

impl MPTExtractionType {
    pub fn from_rlp_node(node: &[u8], i: usize) -> Self {
        let list: Vec<Vec<_>> = rlp::decode_list(node);
        match list.len() {
            // assuming the first node in the path is the leaf
            2 if i == 0 => MPTExtractionType::Leaf,
            2 => MPTExtractionType::Extension,
            // assuming all nodes are valid so branch is the only choice left
            _ => MPTExtractionType::Branch,
        }
    }
}

impl Length {
    pub fn extraction_types(&self) -> Vec<MPTExtractionType> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| MPTExtractionType::from_rlp_node(n, i))
            .collect()
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub block_nr: u64,
    pub storage_root: Vec<u8>,
    pub contract: Address,

    #[dbg(placeholder = "...")]
    pub nodes: Vec<Vec<u8>>,
}

impl Contract {
    pub fn extraction_types(&self) -> Vec<MPTExtractionType> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| MPTExtractionType::from_rlp_node(n, i))
            .collect()
    }
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct BlockExtractionInput {
    #[dbg(placeholder = "...")]
    pub rlp_header: Vec<u8>,
}

impl BlockExtractionInput {
    pub fn new(rlp_header: Vec<u8>) -> Self {
        Self { rlp_header }
    }
}

#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct FinalExtraction {
    pub table_id: u64,

    pub block_nr: u64,

    pub contract: Address,

    /// This is always versioned because we prove values only if they changed.
    pub value_proof_version: MptNodeVersion,

    pub extraction_type: FinalExtractionType,

    #[dbg(placeholder = "...")]
    pub block_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub contract_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub value_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
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

impl From<&WorkerTask> for ProofKey {
    fn from(tt: &WorkerTask) -> Self {
        match &tt.task_type {
            WorkerTaskType::Extraction(ext) => match ext {
                ExtractionType::MptExtraction(mpt) => {
                    let node_version = (mpt.block_nr, mpt.node_hash);
                    match &mpt.mpt_type {
                        MptType::MappingLeaf(_) => {
                            ProofKey::MptVariable(mpt.table_id, node_version)
                        }
                        MptType::MappingBranch(_) => {
                            ProofKey::MptVariable(mpt.table_id, node_version)
                        }
                        MptType::VariableLeaf(_) => {
                            ProofKey::MptVariable(mpt.table_id, node_version)
                        }
                        MptType::VariableBranch(_) => {
                            ProofKey::MptVariable(mpt.table_id, node_version)
                        }
                    }
                }
                ExtractionType::LengthExtraction(length) => {
                    ProofKey::MptLength(length.table_id, length.block_nr)
                }
                ExtractionType::ContractExtraction(contract) => {
                    ProofKey::Contract(contract.contract, contract.block_nr)
                }
                ExtractionType::BlockExtraction(_) => ProofKey::Block(tt.block_nr),
                ExtractionType::FinalExtraction(fe) => {
                    ProofKey::FinalExtraction(fe.table_id, fe.block_nr)
                }
            },
            _ => unimplemented!("WorkerTaskType not implemented"),
        }
    }
}
