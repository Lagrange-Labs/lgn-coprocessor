use derive_debug_plus::Dbg;
use serde_derive::{Deserialize, Serialize};
use verifiable_db::query::aggregation::{ChildPosition, NodeInfo, QueryBounds, SubProof};
use verifiable_db::query::universal_circuit::universal_circuit_inputs::{Placeholders, RowCells};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// What we are proving.
    pub task_type: QueryInputPart,
}

pub struct QueryInput {
    pub pis: Vec<u8>,

    pub parts: QueryInputPart,

    pub aggregation_input_kind: Option<ProofInputKind>,
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInputPart {
    pub embedded_proof_input: Option<EmbeddedProofInput>,

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
    pub left_child_proof_location: Vec<u8>,
    pub right_child_proof_location: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct PartialNodeInput {
    pub proven_child_position: ChildPosition,
    pub proven_child_proof_location: Vec<u8>,
    pub unproven_child_info: Option<NodeInfo>,
    pub is_rows_tree_node: bool,
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
    pub child_location: Vec<u8>,
    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathLeafInput {
    pub node_info: NodeInfo,
    pub left_child_info: Option<NodeInfo>,
    pub right_child_info: Option<NodeInfo>,
    pub is_rows_tree_node: bool,
}
