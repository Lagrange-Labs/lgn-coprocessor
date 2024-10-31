use crate::types::v1::preprocessing::db_keys;
use crate::types::v1::query::keys::ProofKey;
use crate::types::v1::query::{WorkerTask, WorkerTaskType};
use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use serde_derive::{Deserialize, Serialize};
use verifiable_db::query::aggregation::{ChildPosition, NodeInfo};
use verifiable_db::query::universal_circuit::universal_circuit_inputs::{Placeholders, RowCells};

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInput {
    pub proof_key: ProofKey,

    pub query_step: QueryStep,

    #[dbg(placeholder = "...")]
    pub pis: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum QueryStep {
    #[serde(rename = "1")]
    Prepare(Vec<QueryInputPart>),

    #[serde(rename = "2")]
    Revelation(RevelationInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInputPart {
    pub proof_key: ProofKey,

    pub embedded_proof_input: Option<EmbeddedProofInputType>,

    pub aggregation_input_kind: Option<ProofInputKind>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum ProofInputKind {
    /// Match in the end of path or not matched branch
    #[serde(rename = "1")]
    SinglePathLeaf(SinglePathLeafInput),

    /// Match in the middle of path
    #[serde(rename = "2")]
    SinglePathBranch(SinglePathBranchInput),

    /// Node in tree with only one child
    #[serde(rename = "3")]
    PartialNode(PartialNodeInput),

    /// Node in tree with both children
    #[serde(rename = "4")]
    FullNode(FullNodeInput),

    NonExistence(NonExistenceInput),
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct FullNodeInput {
    pub is_rows_tree_node: bool,

    pub left_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub left_child_proof: Vec<u8>,

    pub right_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub right_child_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct PartialNodeInput {
    pub proven_child_position: ChildPosition,

    pub proven_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub proven_child_proof: Vec<u8>,

    pub unproven_child_info: Option<NodeInfo>,

    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum EmbeddedProofInputType {
    #[serde(rename = "1")]
    RowsTree(RowsEmbeddedProofInput),

    #[serde(rename = "2")]
    IndexTree(IndexEmbeddedProofInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct RowsEmbeddedProofInput {
    pub column_cells: RowCells,

    pub placeholders: Placeholders,

    pub is_leaf: bool,
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct IndexEmbeddedProofInput {
    pub rows_proof_key: ProofKey,

    #[dbg(placeholder = "...")]
    pub rows_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathBranchInput {
    pub node_info: NodeInfo,

    pub left_child_info: Option<NodeInfo>,

    pub right_child_info: Option<NodeInfo>,

    pub child_position: ChildPosition,

    pub proven_child_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub proven_child_proof: Vec<u8>,

    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathLeafInput {
    pub node_info: NodeInfo,

    pub left_child_info: Option<NodeInfo>,

    pub right_child_info: Option<NodeInfo>,

    pub is_rows_tree_node: bool,

    pub embedded_proof_location: Option<ProofKey>,

    #[dbg(placeholder = "...")]
    pub embedded_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct RevelationInput {
    pub placeholders: Placeholders,

    pub indexing_proof_location: db_keys::ProofKey,

    pub query_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub indexing_proof: Vec<u8>,

    #[dbg(placeholder = "...")]
    pub query_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct NonExistenceInput {
    pub column_ids: Vec<u64>,

    pub placeholders: Placeholders,

    pub is_rows_tree_node: bool,

    pub node_info: NodeInfo,

    pub left_child_info: Option<NodeInfo>,

    pub right_child_info: Option<NodeInfo>,

    pub primary_index_value: U256,
}

impl From<&WorkerTask> for ProofKey {
    fn from(task: &WorkerTask) -> Self {
        match &task.task_type {
            WorkerTaskType::Query(qr) => qr.proof_key.clone(),
        }
    }
}
