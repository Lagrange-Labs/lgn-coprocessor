use crate::types::v1::query::keys::ProofKey;
use derive_debug_plus::Dbg;
use serde_derive::{Deserialize, Serialize};
use verifiable_db::query::aggregation::{ChildPosition, NodeInfo, QueryBounds, SubProof};
use verifiable_db::query::universal_circuit::universal_circuit_inputs::{Placeholders, RowCells};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInput {
    pub proof_key: ProofKey,

    pub query_step: QueryStep,

    pub pis: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum QueryStep {
    Prepare(Vec<QueryInputPart>),
    Revelation,
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInputPart {
    pub embedded_proof_input: Option<EmbeddedProofInputType>,

    pub aggregation_input_kind: Option<ProofInputKind>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum ProofInputKind {
    /// Match in the end of path or not matched branch
    SinglePathLeaf(SinglePathLeafInput),

    /// Match in the middle of path
    SinglePathBranch(SinglePathBranchInput),

    /// Node in tree with only one child
    PartialNode(PartialNodeInput),

    /// Node in tree with both children
    FullNode(FullNodeInput),
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct FullNodeInput {
    pub is_rows_tree_node: bool,
    pub left_child_proof_location: ProofKey,
    pub right_child_proof_location: ProofKey,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct PartialNodeInput {
    pub proven_child_position: ChildPosition,
    pub proven_child_proof_location: ProofKey,
    pub unproven_child_info: Option<NodeInfo>,
    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum EmbeddedProofInputType {
    RowsTree(EmbeddedProofInput),

    IndexTree(ProofKey),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct EmbeddedProofInput {
    pub column_cells: RowCells,
    pub placeholders: Placeholders,
    pub is_leaf: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathBranchInput {
    pub node_info: NodeInfo,
    pub left_child_info: Option<NodeInfo>,
    pub right_child_info: Option<NodeInfo>,
    pub child_position: ChildPosition,
    pub child_location: ProofKey,
    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathLeafInput {
    pub node_info: NodeInfo,
    pub left_child_info: Option<NodeInfo>,
    pub right_child_info: Option<NodeInfo>,
    pub is_rows_tree_node: bool,
}
