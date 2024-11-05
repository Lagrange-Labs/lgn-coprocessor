use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use verifiable_db::query::aggregation::ChildPosition;
use verifiable_db::query::aggregation::NodeInfo;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::RowCells;
use verifiable_db::revelation::api::MatchingRow;
use verifiable_db::revelation::RowPath;

use crate::types::v1::preprocessing::db_keys;
use crate::types::v1::query::keys::ProofKey;
use crate::types::v1::query::PlaceHolderLgn;
use crate::types::v1::query::WorkerTask;
use crate::types::v1::query::WorkerTaskType;

#[derive(Dbg, Clone, Deserialize, Serialize)]
pub struct QueryInput
{
    pub proof_key: ProofKey,

    pub query_step: QueryStep,

    #[dbg(placeholder = "...")]
    pub pis: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QueryStep
{
    #[serde(rename = "1")]
    Prepare(Vec<QueryInputPart>),

    #[serde(rename = "2")]
    Revelation(RevelationInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct QueryInputPart
{
    pub proof_key: ProofKey,

    pub embedded_proof_input: Option<EmbeddedProofInputType>,

    pub aggregation_input_kind: Option<ProofInputKind>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum ProofInputKind
{
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
pub struct FullNodeInput
{
    pub is_rows_tree_node: bool,

    pub left_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub left_child_proof: Vec<u8>,

    pub right_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub right_child_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct PartialNodeInput
{
    pub proven_child_position: ChildPosition,

    pub proven_child_proof_location: ProofKey,

    #[dbg(placeholder = "...")]
    pub proven_child_proof: Vec<u8>,

    pub unproven_child_info: Option<NodeInfo>,

    pub is_rows_tree_node: bool,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum EmbeddedProofInputType
{
    #[serde(rename = "1")]
    RowsTree(RowsEmbeddedProofInput),

    #[serde(rename = "2")]
    IndexTree(IndexEmbeddedProofInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct RowsEmbeddedProofInput
{
    pub column_cells: RowCells,

    pub placeholders: PlaceHolderLgn,

    pub is_leaf: bool,
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct IndexEmbeddedProofInput
{
    pub rows_proof_key: ProofKey,

    #[dbg(placeholder = "...")]
    pub rows_proof: Vec<u8>,
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct SinglePathBranchInput
{
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
pub struct SinglePathLeafInput
{
    pub node_info: NodeInfo,

    pub left_child_info: Option<NodeInfo>,

    pub right_child_info: Option<NodeInfo>,

    pub is_rows_tree_node: bool,

    pub embedded_proof_location: Option<ProofKey>,

    #[dbg(placeholder = "...")]
    pub embedded_proof: Vec<u8>,
}

#[derive(Clone, Dbg, Serialize, Deserialize)]
pub struct HydratableMatchingRow
{
    pub proof: Hydratable<db_keys::ProofKey>,
    pub path: RowPath,
    pub result: Vec<U256>,
}
impl HydratableMatchingRow
{
    pub fn into_matching_row(self) -> MatchingRow
    {
        MatchingRow::new(
            self.proof
                .into_proof(),
            self.path,
            self.result,
        )
    }
}

/// Either a `Dehydrated` variant containing a key to a stored proof, or a
/// `Hydrated` containing the proof itself.
#[derive(Clone, Serialize, Deserialize)]
pub enum Hydratable<K: Clone + std::fmt::Debug>
{
    Dehydrated(K),
    Hydrated(Vec<u8>),
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Hydratable<T>
{
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result
    {
        match self
        {
            Hydratable::Dehydrated(k) =>
            {
                write!(
                    f,
                    "dehydrated: {k:?}"
                )
            },
            Hydratable::Hydrated(_) =>
            {
                write!(
                    f,
                    "hydrated"
                )
            },
        }
    }
}

impl<K: Clone + std::fmt::Debug> Hydratable<K>
{
    /// Wrap a proof key into a `Dehydrated` variant.
    pub fn new(k: K) -> Self
    {
        Hydratable::Dehydrated(k)
    }

    /// Consume a `Hydrated` variant into its embedded proof; panic if it is
    /// not hydrated.
    pub fn into_proof(self) -> Vec<u8>
    {
        match self
        {
            Hydratable::Dehydrated(_) => unreachable!(),
            Hydratable::Hydrated(proof) => proof,
        }
    }

    /// Convert a `Dehydrated` variant into its embedded key; panic if it is
    /// not hydrated.
    pub fn key(&self) -> K
    {
        match self
        {
            Hydratable::Dehydrated(k) => k.clone(),
            Hydratable::Hydrated(_) => unreachable!(),
        }
    }

    /// Hydrates a `Dehydrated` variant; panic if it is already hydrated.
    pub fn hydrate(
        &mut self,
        proof: Vec<u8>,
    )
    {
        assert!(
            matches!(
                self,
                Hydratable::Dehydrated(_)
            )
        );
        *self = Hydratable::Hydrated(proof)
    }
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub enum RevelationInput
{
    Aggregated
    {
        placeholders: PlaceHolderLgn,

        indexing_proof_location: db_keys::ProofKey,

        query_proof_location: ProofKey,

        #[dbg(placeholder = "...")]
        indexing_proof: Vec<u8>,

        #[dbg(placeholder = "...")]
        query_proof: Vec<u8>,
    },
    Tabular
    {
        placeholders: PlaceHolderLgn,
        indexing_proof: Hydratable<db_keys::ProofKey>,
        matching_rows: Vec<HydratableMatchingRow>,
        column_ids: ColumnIDs,
        limit: u64,
        offset: u64,
    },
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct NonExistenceInput
{
    pub column_ids: Vec<u64>,

    pub placeholders: PlaceHolderLgn,

    pub is_rows_tree_node: bool,

    pub node_info: NodeInfo,

    pub left_child_info: Option<NodeInfo>,

    pub right_child_info: Option<NodeInfo>,

    pub primary_index_value: U256,
}

impl From<&WorkerTask> for ProofKey
{
    fn from(task: &WorkerTask) -> Self
    {
        match &task.task_type
        {
            WorkerTaskType::Query(qr) =>
            {
                qr.proof_key
                    .clone()
            },
        }
    }
}
