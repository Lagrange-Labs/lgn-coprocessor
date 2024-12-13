use std::sync::Arc;

use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use verifiable_db::query::api::RowInput;
use verifiable_db::query::api::TreePathInputs;
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

#[derive(Dbg, Clone, Deserialize, Serialize)]
pub enum QueryInputPart
{
    #[serde(rename = "1")]
    Aggregation(
        ProofKey,
        Box<ProofInputKind>,
    ),

    // We only need to handle rows tree proving for now.
    #[serde(rename = "2")]
    Embedded(
        ProofKey,
        EmbeddedProofInputType,
    ),
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub enum ProofInputKind
{
    #[serde(rename = "1")]
    RowsChunk(RowsChunkInput),

    #[serde(rename = "2")]
    ChunkAggregation(ChunkAggregationInput),

    #[serde(rename = "3")]
    NonExistence(Box<NonExistenceInput>),
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub enum EmbeddedProofInputType
{
    #[serde(rename = "1")]
    RowsTree(RowsEmbeddedProofInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct RowsEmbeddedProofInput
{
    pub column_cells: RowCells,

    pub placeholders: PlaceHolderLgn,

    pub is_leaf: bool,
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
                .clone_proof(),
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
    Hydrated(Arc<Vec<u8>>),
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
    pub fn proof(&self) -> Arc<Vec<u8>>
    {
        match self
        {
            Hydratable::Dehydrated(_) => unreachable!(),
            Hydratable::Hydrated(proof) => proof.clone(),
        }
    }

    /// Consume a `Hydrated` variant into its embedded proof; panic if it is
    /// not hydrated.
    pub fn clone_proof(&self) -> Vec<u8>
    {
        match self
        {
            Hydratable::Dehydrated(_) => unreachable!(),
            Hydratable::Hydrated(proof) =>
            {
                proof
                    .clone()
                    .to_vec()
            },
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
        *self = Hydratable::Hydrated(Arc::new(proof))
    }
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub enum RevelationInput
{
    Aggregated
    {
        placeholders: PlaceHolderLgn,

        #[dbg(placeholder = "...")]
        // Used in DQ
        #[allow(unused_variables)]
        indexing_proof: Hydratable<db_keys::ProofKey>,

        #[dbg(placeholder = "...")]
        // Used in DQ
        #[allow(unused_variables)]
        query_proof: Hydratable<ProofKey>,
    },
    Tabular
    {
        placeholders: PlaceHolderLgn,
        indexing_proof: Hydratable<db_keys::ProofKey>,
        matching_rows: Vec<HydratableMatchingRow>,
        column_ids: ColumnIDs,
        limit: u32,
        offset: u32,
    },
}

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct NonExistenceInput
{
    pub index_path: TreePathInputs,

    pub column_ids: Vec<u64>,

    pub placeholders: PlaceHolderLgn,
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

#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct RowsChunkInput
{
    pub rows: Vec<RowInput>,

    pub placeholders: PlaceHolderLgn,
}

#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct ChunkAggregationInput
{
    pub child_proofs: Vec<Hydratable<ProofKey>>,
}
