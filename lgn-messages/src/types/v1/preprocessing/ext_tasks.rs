use alloy::primitives::Address;
use alloy::primitives::FixedBytes;
use alloy::primitives::U256;
use derive_debug_plus::Dbg;
use mp2_common::eth::node_type;
use mp2_common::eth::NodeType;
use mp2_v1::api::TableRow;
use mp2_v1::final_extraction::OffChainRootOfTrust;
use mp2_v1::indexing::cell::Cell;
use mp2_v1::indexing::ColumnID;
use mp2_v1::values_extraction::gadgets::column_info::ColumnInfo;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::preprocessing::ext_keys::ProofKey;
use crate::types::v1::preprocessing::WorkerTask;
use crate::types::v1::preprocessing::WorkerTaskType;
use crate::BlockNr;
use crate::MptNodeVersion;
use crate::TableHash;
use crate::TableId;

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
    pub node_hash: FixedBytes<32>,
    pub mpt_type: MptType,
}

impl Mpt {
    pub fn new(
        table_hash: TableId,
        block_nr: BlockNr,
        node_hash: FixedBytes<32>,
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
    pub evm_word: u32,
    pub table_info: Vec<ColumnInfo>,
}

impl MappingLeafInput {
    pub fn new(
        key: Vec<u8>,
        node: Vec<u8>,
        slot: u8,
        key_id: u64,
        evm_word: u32,
        table_info: Vec<ColumnInfo>,
    ) -> Self {
        Self {
            key,
            node,
            slot,
            key_id,
            evm_word,
            table_info,
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
    pub evm_word: u32,
    pub table_info: Vec<ColumnInfo>,
}

impl VariableLeafInput {
    pub fn new(
        node: Vec<u8>,
        slot: u8,
        evm_word: u32,
        table_info: Vec<ColumnInfo>,
    ) -> Self {
        Self {
            node,
            slot,
            evm_word,
            table_info,
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
        match node_type(node).unwrap() {
            // assuming the first node in the path is the leaf
            NodeType::Leaf if i == 0 => MPTExtractionType::Leaf,
            NodeType::Leaf | NodeType::Extension => MPTExtractionType::Extension,
            // assuming all nodes are valid so branch is the only choice left
            NodeType::Branch => MPTExtractionType::Branch,
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

/// Inputs for the final extraction.
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub enum FinalExtraction {
    Single(SingleTableExtraction),
    Merge(MergeTableExtraction),
    Offchain(OffchainExtraction),
}

impl FinalExtraction {
    fn table_id(&self) -> BlockNr {
        match self {
            FinalExtraction::Single(single_table_extraction) => single_table_extraction.table_id,
            FinalExtraction::Merge(merge_table_extraction) => merge_table_extraction.table_id,
            FinalExtraction::Offchain(offchain_extraction) => offchain_extraction.table_id,
        }
    }

    fn block_nr(&self) -> BlockNr {
        match self {
            FinalExtraction::Single(single_table_extraction) => single_table_extraction.block_nr,
            FinalExtraction::Merge(merge_table_extraction) => merge_table_extraction.block_nr,
            FinalExtraction::Offchain(offchain_extraction) => {
                offchain_extraction
                    .primary_index
                    .try_into()
                    // Should not happen, a u64 should be more than sufficient to represent the primary indexes
                    .unwrap_or_default()
            },
        }
    }

    pub fn new_single_table(
        table_id: TableId,
        table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        extraction_type: FinalExtractionType,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        Self::Single(SingleTableExtraction::new(
            table_id,
            table_hash,
            block_nr,
            contract,
            extraction_type,
            value_proof_version,
        ))
    }

    pub fn new_merge_table(
        table_id: TableId,
        simple_table_hash: TableHash,
        mapping_table_hash: TableHash,
        block_nr: BlockNr,
        contract: Address,
        value_proof_version: MptNodeVersion,
    ) -> Self {
        Self::Merge(MergeTableExtraction::new(
            table_id,
            simple_table_hash,
            mapping_table_hash,
            block_nr,
            contract,
            value_proof_version,
        ))
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
        extraction_type: FinalExtractionType,
        value_proof_version: MptNodeVersion,
    ) -> Self {
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
    Simple,
    Lengthed,
}

/// Wrapper structure used to change the serialisation format.
///
/// The `TableRow` data type uses a map indexed by u64, this normally
/// works with serde_json, except if a parent container uses a tagged
/// union, this is an unresolved bug in serde.
///
/// This struct exists to change the map to a vec, to circunvent the above.
///
/// issue: https://github.com/serde-rs/serde/issues/1183
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ExtractionRow {
    pub primary_index_column: Cell,
    pub other_columns: Vec<Cell>,
}

impl From<ExtractionRow> for TableRow {
    fn from(value: ExtractionRow) -> Self {
        TableRow::new(value.primary_index_column, value.other_columns)
    }
}

/// Inputs for an off-chain extraction.
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct OffchainExtraction {
    /// Determines which table this extraction corresponds to.
    pub table_id: TableId,

    /// The "revision" of this extraction, also called `block_nr`.
    pub revision: BlockNr,

    /// Determines the previous extraction block_nr/revision.
    ///
    /// Determines the proof that should go to `prev_epoch_proof`.
    pub previous_epoch: Option<BlockNr>,

    /// The value of the primary index, akin to the block_nr value.
    pub primary_index: U256,

    /// Determines if the data should have its commitment computed/verified.
    pub root_of_trust: OffChainRootOfTrust,

    /// The previous proof.
    pub prev_epoch_proof: Option<Vec<u8>>,

    /// The data to be extracted.
    pub table_rows: Vec<ExtractionRow>,

    /// Determines the shape of the data.
    pub row_unique_columns: Vec<ColumnID>,
}

impl From<&WorkerTask> for ProofKey {
    fn from(task: &WorkerTask) -> Self {
        match &task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt_extraction) => {
                        let node_version = (mpt_extraction.block_nr, mpt_extraction.node_hash);
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
