#![feature(generic_const_exprs)]

use std::fmt::Display;
use std::fmt::Formatter;

use ethers::types::H256;
use mp2_v1::contract_extraction;
use mp2_v1::final_extraction;
use mp2_v1::length_extraction;
use mp2_v1::values_extraction;
use serde::Deserialize;
use serde::Serialize;
use v1::preprocessing::ConcreteValueExtractionCircuitInput;
use v1::query::ConcretInnerQueryCircuitInput;
use v1::query::ConcreteQueryCircuitInput;
use v1::query::ConcreteRevelationCircuitInput;
use v1::query::ConcreteUniversalCircuit;
use v1::ConcreteCircuitInput;
use verifiable_db::block_tree;
use verifiable_db::cells_tree;
use verifiable_db::ivc;
use verifiable_db::query::universal_circuit::universal_query_circuit;
use verifiable_db::revelation;
use verifiable_db::row_tree;

pub mod v1;

pub type BlockNr = u64;
pub type TableId = u64;
pub type TableHash = u64;
pub type ChainId = u64;
pub type Proof = Vec<u8>;
pub type QueryId = String;
pub type RowKeyId = String;
pub type Identifier = u64;
pub type MptNodeVersion = (BlockNr, H256);

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "version")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// Version 1 of the envelope format
    #[serde(rename = "1")]
    V1(v1::Envelope),

    /// Used by serde if the payload's version tag is not known.
    #[serde(other)]
    Unsupported,
}

impl Message {
    /// Creates a message using the `v1` format.
    pub fn v1(
        task_id: String,
        task: v1::Task,
        version: String,
    ) -> Self {
        Self::V1(v1::Envelope {
            task,
            task_id,
            mp2_version: version,
        })
    }

    /// Returns the task identifier.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Message::V1(v1::Envelope { task_id, .. }) => Some(task_id),
            Message::Unsupported => None,
        }
    }

    /// Returns this message's task.
    pub fn task(&self) -> Option<&v1::Task> {
        match self {
            Message::V1(v1::Envelope { task, .. }) => Some(task),
            Message::Unsupported => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "version")]
#[serde(rename_all = "snake_case")]
pub enum Response {
    #[serde(rename = "1")]
    V1(v1::ReplyEnvelope),
}

impl Response {
    pub fn v1(
        task_id: String,
        proof: Proof,
    ) -> Self {
        Response::V1(v1::ReplyEnvelope::Proof { task_id, proof })
    }
}

/// The segregation of job types according to their computational complexity
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskDifficulty {
    // Due to the implicit ordering on which PartialOrd is built, this **MUST**
    // remain the smaller value at the top of the enum.
    // Hence, all workers of this class will always test .LT. *all* the tasks in
    // queue.
    /// Accept no tasks
    Disabled,
    /// Accept S tasks
    Small,
    /// Accept M tasks
    Medium,
    /// Accept L tasks
    Large,
}

impl Display for TaskDifficulty {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TaskDifficulty::Small => "small",
                TaskDifficulty::Medium => "medium",
                TaskDifficulty::Large => "large",
                TaskDifficulty::Disabled => "disabled",
            }
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProverType {
    Unsupported,
    V1Preprocessing,
    V1Query,
    V1Groth16,
}

impl Display for ProverType {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProverType::Unsupported => "Unsupported",
                ProverType::V1Preprocessing => "V1Preprocessing",
                ProverType::V1Query => "V1Query",
                ProverType::V1Groth16 => "V1Groth16",
            }
        )
    }
}

pub trait ToProverType {
    fn to_prover_type(&self) -> ProverType;
}

const UNSUPPORTED: &str = "unsupported";
const GROTH16: &str = "groth16";
const BATCHED_INDEX_EXTRACTION: &str = "batched_index_extraction";
const BATCHED_LENGTH_EXTRACTION: &str = "batched_length_extraction";
const BATCHED_CONTRACT_EXTRACTION: &str = "batched_contract_extraction";
const BLOCK_EXTRACTION: &str = "block_extraction";
const CONTRACT_LEAF_EXTRACTION: &str = "contract_leaf_extraction";
const CONTRACT_EXTENSION_EXTRACTION: &str = "contract_extension_extraction";
const CONTRACT_BRANCH_EXTRACTION: &str = "contract_branch_extraction";
const LENGTH_BRANCH_EXTRACTION: &str = "length_branch_extraction";
const LENGTH_EXTENSION_EXTRACTION: &str = "length_extension_extraction";
const LENGTH_LEAF_EXTRACTION: &str = "length_leaf_extraction";
const SINGLE_VALUE_LEAF_EXTRACTION: &str = "single_value_leaf_extraction";
const MAPPING_VALUE_LEAF_EXTRACTION: &str = "mapping_value_leaf_extraction";
const MAPPING_OF_MAPPING_LEAF_EXTRACTION: &str = "mapping_of_mapping_leaf_extraction";
const VALUE_EXTENSION_EXTRACTION: &str = "value_extension_extraction";
const VALUE_BRANCH_EXTRACTION: &str = "value_branch_extraction";
const FINAL_SIMPLE_EXTRACTION: &str = "final_simple_extraction";
const FINAL_LENGTHED_EXTRACTION: &str = "final_lengthed_extraction";
const FINAL_MERGE_EXTRACTION: &str = "final_merge_extraction";
const FINAL_DUMMY_EXTRACTION: &str = "final_dummy_extraction";
const CELLS_TREE_LEAF_EXTRACTION: &str = "cells_tree_leaf_extraction";
const CELLS_TREE_FULL_NODE_EXTRACTION: &str = "cells_tree_full_node_extraction";
const CELLS_TREE_PARTIAL_NODE_EXTRACTION: &str = "cells_tree_partial_node_extraction";
const ROW_TREE_LEAF_EXTRACTION: &str = "row_tree_leaf_extraction";
const ROW_TREE_FULL_EXTRACTION: &str = "row_tree_full_extraction";
const ROW_TREE_PARTIAL_EXTRACTION: &str = "row_tree_partial_extraction";
const BLOCK_TREE_LEAF_EXTRACTION: &str = "block_tree_leaf_extraction";
const BLOCK_TREE_PARENT_EXTRACTION: &str = "block_tree_parent_extraction";
const BLOCK_TREE_MEMBERSHIP_EXTRACTION: &str = "block_tree_membership_extraction";
const IVC_FIRST_PROOF: &str = "ivc_first_proof";
const IVC_SUBSEQUENT_PROOF: &str = "ivc_subsequent_proof";
const QUERY_BATCHED_TABULAR: &str = "query_batched_tabular";
const QUERY_ROW_CHUNK_WITH_AGGREGATION: &str = "query_row_chunk_with_aggregation";
const QUERY_CHUNK_AGGREGATION: &str = "query_chunk_aggregation";
const QUERY_NON_EXISTENCE: &str = "query_non_existence";
const UNIVERSAL_QUERY_WITH_AGGREGATION: &str = "universal_query_with_aggregation";
const UNIVERSAL_QUERY_NO_AGGREGATION: &str = "universal_query_no_aggregation";
const REVELATION_NO_RESULTS_TREE: &str = "revelation_no_results_tree";
const REVELATION_UNPROVEN_OFFSET: &str = "revelation_unproven_offset";

/// Used to map a message to a class name.
///
/// This information is used for debugging and metrics.
pub trait ToMessageClass {
    fn message_class(&self) -> &'static str;
}

impl ToMessageClass for Message {
    fn message_class(&self) -> &'static str {
        match self {
            Message::V1(envelope) => envelope.message_class(),
            Message::Unsupported => UNSUPPORTED,
        }
    }
}

impl ToMessageClass for v1::Envelope {
    fn message_class(&self) -> &'static str {
        match &self.task {
            v1::Task::Preprocessing(preprocessing_task) => preprocessing_task.message_class(),
            v1::Task::Query(query_task) => query_task.message_class(),
            v1::Task::Groth16(..) => GROTH16,
        }
    }
}

impl ToMessageClass for v1::preprocessing::PreprocessingTask {
    fn message_class(&self) -> &'static str {
        match self {
            v1::preprocessing::PreprocessingTask::BatchedIndex(_batched_index) => {
                BATCHED_INDEX_EXTRACTION
            },
            v1::preprocessing::PreprocessingTask::BatchedLength(_batched_length) => {
                BATCHED_LENGTH_EXTRACTION
            },
            v1::preprocessing::PreprocessingTask::BatchedContract(_batched_contract) => {
                BATCHED_CONTRACT_EXTRACTION
            },
            v1::preprocessing::PreprocessingTask::CircuitInput(circuit_input) => {
                circuit_input.message_class()
            },
        }
    }
}

impl ToMessageClass for ConcreteCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            mp2_v1::api::CircuitInput::ContractExtraction(circuit_input) => {
                circuit_input.message_class()
            },
            mp2_v1::api::CircuitInput::LengthExtraction(length_circuit_input) => {
                length_circuit_input.message_class()
            },
            mp2_v1::api::CircuitInput::ValuesExtraction(circuit_input) => {
                circuit_input.message_class()
            },
            mp2_v1::api::CircuitInput::BlockExtraction(_circuit_input) => BLOCK_EXTRACTION,
            mp2_v1::api::CircuitInput::FinalExtraction(circuit_input) => {
                circuit_input.message_class()
            },
            mp2_v1::api::CircuitInput::CellsTree(circuit_input) => circuit_input.message_class(),
            mp2_v1::api::CircuitInput::RowsTree(circuit_input) => circuit_input.message_class(),
            mp2_v1::api::CircuitInput::BlockTree(circuit_input) => circuit_input.message_class(),
            mp2_v1::api::CircuitInput::IVC(circuit_input) => circuit_input.message_class(),
        }
    }
}

impl ToMessageClass for contract_extraction::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            contract_extraction::CircuitInput::Leaf(_leaf_circuit) => CONTRACT_LEAF_EXTRACTION,
            contract_extraction::CircuitInput::Extension(_proof_input_serialized) => {
                CONTRACT_EXTENSION_EXTRACTION
            },
            contract_extraction::CircuitInput::Branch(_proof_input_serialized) => {
                CONTRACT_BRANCH_EXTRACTION
            },
        }
    }
}

impl ToMessageClass for length_extraction::LengthCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            length_extraction::LengthCircuitInput::Branch(_proof_input_serialized) => {
                LENGTH_BRANCH_EXTRACTION
            },
            length_extraction::LengthCircuitInput::Extension(_proof_input_serialized) => {
                LENGTH_EXTENSION_EXTRACTION
            },
            length_extraction::LengthCircuitInput::Leaf(_leaf_length_circuit) => {
                LENGTH_LEAF_EXTRACTION
            },
        }
    }
}

impl ToMessageClass for ConcreteValueExtractionCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            values_extraction::CircuitInput::LeafSingle(_leaf_single_circuit) => {
                SINGLE_VALUE_LEAF_EXTRACTION
            },
            values_extraction::CircuitInput::LeafMapping(_leaf_mapping_circuit) => {
                MAPPING_VALUE_LEAF_EXTRACTION
            },
            values_extraction::CircuitInput::LeafMappingOfMappings(
                _leaf_mapping_of_mappings_circuit,
            ) => MAPPING_OF_MAPPING_LEAF_EXTRACTION,
            values_extraction::CircuitInput::Extension(_proof_input_serialized) => {
                VALUE_EXTENSION_EXTRACTION
            },
            values_extraction::CircuitInput::Branch(_proof_input_serialized) => {
                VALUE_BRANCH_EXTRACTION
            },
        }
    }
}

impl ToMessageClass for final_extraction::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            final_extraction::CircuitInput::Simple(_simple_circuit_input) => {
                FINAL_SIMPLE_EXTRACTION
            },
            final_extraction::CircuitInput::Lengthed(_lengthed_circuit_input) => {
                FINAL_LENGTHED_EXTRACTION
            },
            final_extraction::CircuitInput::MergeTable(_merge_circuit_input) => {
                FINAL_MERGE_EXTRACTION
            },
            final_extraction::CircuitInput::NoProvable(_dummy_circuit) => FINAL_DUMMY_EXTRACTION,
        }
    }
}

impl ToMessageClass for cells_tree::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            cells_tree::CircuitInput::Leaf(_leaf_circuit) => CELLS_TREE_LEAF_EXTRACTION,
            cells_tree::CircuitInput::FullNode(_proof_input_serialized) => {
                CELLS_TREE_FULL_NODE_EXTRACTION
            },
            cells_tree::CircuitInput::PartialNode(_proof_input_serialized) => {
                CELLS_TREE_PARTIAL_NODE_EXTRACTION
            },
        }
    }
}

impl ToMessageClass for row_tree::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            row_tree::CircuitInput::Leaf { .. } => ROW_TREE_LEAF_EXTRACTION,
            row_tree::CircuitInput::Full { .. } => ROW_TREE_FULL_EXTRACTION,
            row_tree::CircuitInput::Partial { .. } => ROW_TREE_PARTIAL_EXTRACTION,
        }
    }
}

impl ToMessageClass for block_tree::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            block_tree::CircuitInput::Leaf { .. } => BLOCK_TREE_LEAF_EXTRACTION,
            block_tree::CircuitInput::Parent { .. } => BLOCK_TREE_PARENT_EXTRACTION,
            block_tree::CircuitInput::Membership { .. } => BLOCK_TREE_MEMBERSHIP_EXTRACTION,
        }
    }
}

impl ToMessageClass for ivc::CircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            ivc::CircuitInput::FirstProof { .. } => IVC_FIRST_PROOF,
            ivc::CircuitInput::SubsequentProof { .. } => IVC_SUBSEQUENT_PROOF,
        }
    }
}

impl ToMessageClass for v1::query::QueryTask {
    fn message_class(&self) -> &'static str {
        match self {
            v1::query::QueryTask::QueryCircuitInput(query_circuit_input) => {
                query_circuit_input.message_class()
            },
            v1::query::QueryTask::BatchedTabular { .. } => QUERY_BATCHED_TABULAR,
        }
    }
}

impl ToMessageClass for ConcreteQueryCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            verifiable_db::api::QueryCircuitInput::Query(circuit_input) => {
                circuit_input.message_class()
            },
            verifiable_db::api::QueryCircuitInput::Revelation(circuit_input) => {
                circuit_input.message_class()
            },
        }
    }
}

impl ToMessageClass for ConcretInnerQueryCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            verifiable_db::query::api::CircuitInput::RowChunkWithAggregation(
                _row_chunk_processing_circuit,
            ) => QUERY_ROW_CHUNK_WITH_AGGREGATION,
            verifiable_db::query::api::CircuitInput::ChunkAggregation(
                _chunk_aggregation_inputs,
            ) => QUERY_CHUNK_AGGREGATION,
            verifiable_db::query::api::CircuitInput::NonExistence(_non_existence_circuit) => {
                QUERY_NON_EXISTENCE
            },
            verifiable_db::query::api::CircuitInput::UniversalCircuit(universal_circuit_input) => {
                universal_circuit_input.message_class()
            },
        }
    }
}

impl ToMessageClass for ConcreteUniversalCircuit {
    fn message_class(&self) -> &'static str {
        match self {
            universal_query_circuit::UniversalCircuitInput::QueryWithAgg(
                _universal_query_circuit_inputs,
            ) => UNIVERSAL_QUERY_WITH_AGGREGATION,
            universal_query_circuit::UniversalCircuitInput::QueryNoAgg(
                _universal_query_circuit_inputs,
            ) => UNIVERSAL_QUERY_NO_AGGREGATION,
        }
    }
}

impl ToMessageClass for ConcreteRevelationCircuitInput {
    fn message_class(&self) -> &'static str {
        match self {
            revelation::api::CircuitInput::NoResultsTree { .. } => REVELATION_NO_RESULTS_TREE,
            revelation::api::CircuitInput::UnprovenOffset { .. } => REVELATION_UNPROVEN_OFFSET,
        }
    }
}
