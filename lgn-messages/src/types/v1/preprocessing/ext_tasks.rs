use alloy_primitives::Address;
use derive_debug_plus::Dbg;
use ethers::types::H256;
use ethers::utils::rlp;
use mp2_common::digest::TableDimension;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::preprocessing::ext_keys::ProofKey;
use crate::types::v1::preprocessing::WorkerTask;
use crate::types::v1::preprocessing::WorkerTaskType;
use crate::BlockNr;
use crate::TableHash;
use crate::TableId;

pub const ROUTING_DOMAIN: &str = "sp";
pub type Identifier = u64;
pub type MptNodeVersion = (
    BlockNr,
    H256,
);

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
    FinalExtraction(Box<FinalExtraction>),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Mpt {
    pub table_hash: TableHash,
    pub block_nr: BlockNr,
    pub node_hash: H256,
    pub mpt_type: MptType,
}

impl Mpt {
    pub fn new(
        table_hash: TableId,
        block_nr: BlockNr,
        node_hash: H256,
        mpt_type: MptType,
    ) -> Self {
        Self {
            table_hash,
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
        key_id: u64,
        value_id: u64,
    ) -> Self {
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
    pub fn new(
        node: Vec<u8>,
        children: Vec<MptNodeVersion>,
    ) -> Self {
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
    pub fn new(
        node: Vec<u8>,
        slot: u8,
        column_id: u64,
    ) -> Self {
        Self {
            node,
            slot,
            column_id,
        }
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableBranchInput {
    pub table_id: TableId,
    pub node: Vec<u8>,
    pub children: Vec<MptNodeVersion>,

    #[dbg(placeholder = "...")]
    pub children_proofs: Vec<Vec<u8>>,
}

impl VariableBranchInput {
    pub fn new(
        table_id: TableId,
        node: Vec<u8>,
        children: Vec<MptNodeVersion>,
    ) -> Self {
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
    pub table_hash: TableHash,
    pub block_nr: BlockNr,
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
    pub fn from_rlp_node(
        node: &[u8],
        i: usize,
    ) -> Self {
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
            .map(
                |(i, n)| {
                    MPTExtractionType::from_rlp_node(
                        n,
                        i,
                    )
                },
            )
            .collect()
    }
}

#[derive(Dbg, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Contract {
    pub block_nr: BlockNr,
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
            .map(
                |(i, n)| {
                    MPTExtractionType::from_rlp_node(
                        n,
                        i,
                    )
                },
            )
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
        Self {
            rlp_header,
        }
    }
}

/// Inputs for the final extraction.
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub enum FinalExtraction {
    Single(SingleTableExtraction),
    Merge(MergeTableExtraction),
}

impl FinalExtraction {
    fn table_id(&self) -> BlockNr {
        match self {
            FinalExtraction::Single(single_table_extraction) => single_table_extraction.table_id,
            FinalExtraction::Merge(merge_table_extraction) => merge_table_extraction.table_id,
        }
    }

    fn block_nr(&self) -> BlockNr {
        match self {
            FinalExtraction::Single(single_table_extraction) => single_table_extraction.block_nr,
            FinalExtraction::Merge(merge_table_extraction) => merge_table_extraction.block_nr,
        }
    }

    pub fn new_single_table(
        table_id: TableId,
        table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        compound: Option<TableDimension>,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        Self::Single(
            SingleTableExtraction::new(
                table_id,
                table_hash,
                block_nr,
                contract,
                compound,
                value_proof_version,
            ),
        )
    }

    pub fn new_merge_table(
        table_id: TableId,
        simple_table_hash: TableHash,
        mapping_table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        Self::Merge(
            MergeTableExtraction::new(
                table_id,
                simple_table_hash,
                mapping_table_hash,
                block_nr,
                contract,
                value_proof_version,
            ),
        )
    }
}

/// Inputs for a single table proof.
///
/// # Identifiers
///
/// A [SingleTableExtraction] is either a final which binds together a block, contract, and a
/// table. The table may be either a simple, mapping, or mapping with length
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct SingleTableExtraction {
    pub table_id: TableId,
    pub table_hash: TableHash,
    pub value_proof_version: MptNodeVersion,
    pub block_nr: BlockNr,
    pub contract: Address,
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

impl SingleTableExtraction {
    pub fn new(
        table_id: TableId,
        table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        compound: Option<TableDimension>,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        let extraction_type = match compound {
            Some(compound) => FinalExtractionType::Simple(compound),
            None => FinalExtractionType::Lengthed,
        };

        Self {
            table_id,
            table_hash,
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

/// Inputs for a merge table proof.
///
/// # Identifiers
///
/// A [MergeTableExtraction] is a final extraction which binds together a block, contract, and its
/// two sub-tables.
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct MergeTableExtraction {
    pub table_id: TableId,
    pub simple_table_hash: TableHash,
    pub mapping_table_hash: TableHash,
    pub block_nr: BlockNr,
    pub contract: Address,

    /// Determines the version of the storage node.
    ///
    /// The version is determined by the last block_nr at which the storage changed, and its hash.
    /// A single value is necessary for the simple and mapping tables because the data comes from
    /// the same contract.
    pub value_proof_version: MptNodeVersion,

    #[dbg(placeholder = "...")]
    pub block_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub contract_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub simple_table_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub mapping_table_proof: Vec<u8>,
}

impl MergeTableExtraction {
    pub fn new(
        table_id: TableId,
        simple_table_hash: TableHash,
        mapping_table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        Self {
            table_id,
            simple_table_hash,
            mapping_table_hash,
            block_nr,
            contract,
            value_proof_version,
            block_proof: vec![],
            contract_proof: vec![],
            simple_table_proof: vec![],
            mapping_table_proof: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum FinalExtractionType {
    Simple(TableDimension),
    Lengthed,
}

impl From<&WorkerTask> for ProofKey {
    fn from(task: &WorkerTask) -> Self {
        match &task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt_extraction) => {
                        let node_version = (
                            mpt_extraction.block_nr,
                            mpt_extraction.node_hash,
                        );
                        match &mpt_extraction.mpt_type {
                            MptType::MappingLeaf(_) => {
                                ProofKey::MptVariable {
                                    table_hash: mpt_extraction.table_hash,
                                    mpt_node_version: node_version,
                                }
                            },
                            MptType::MappingBranch(_) => {
                                ProofKey::MptVariable {
                                    table_hash: mpt_extraction.table_hash,
                                    mpt_node_version: node_version,
                                }
                            },
                            MptType::VariableLeaf(_) => {
                                ProofKey::MptVariable {
                                    table_hash: mpt_extraction.table_hash,
                                    mpt_node_version: node_version,
                                }
                            },
                            MptType::VariableBranch(_) => {
                                ProofKey::MptVariable {
                                    table_hash: mpt_extraction.table_hash,
                                    mpt_node_version: node_version,
                                }
                            },
                        }
                    },
                    ExtractionType::LengthExtraction(length) => {
                        ProofKey::MptLength {
                            table_hash: length.table_hash,
                            block_nr: length.block_nr,
                        }
                    },
                    ExtractionType::ContractExtraction(contract) => {
                        ProofKey::Contract {
                            address: contract.contract,
                            block_nr: contract.block_nr,
                        }
                    },
                    ExtractionType::BlockExtraction(_) => {
                        ProofKey::Block {
                            block_nr: task.block_nr,
                        }
                    },
                    ExtractionType::FinalExtraction(final_extraction) => {
                        ProofKey::FinalExtraction {
                            table_id: final_extraction.table_id(),
                            block_nr: final_extraction.block_nr(),
                        }
                    },
                }
            },
            _ => unimplemented!("WorkerTaskType not implemented"),
        }
    }
}
