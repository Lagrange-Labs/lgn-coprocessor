#![allow(unused_variables)]
use std::sync::Arc;

use alloy::primitives::U256;
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

/// Query input for a proving task
#[derive(Dbg, Clone, Deserialize, Serialize)]
pub struct QueryInput {
    /// Proof storage key
    pub proof_key: ProofKey,

    /// Query step info
    pub query_step: QueryStep,

    /// Public inputs data
    #[dbg(placeholder = "...")]
    pub pis: Vec<u8>,
}

/// Query step info
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum QueryStep {
    /// Combine the rows and revelation proving for tabular queries in one task,
    /// next step is Groth16
    #[serde(rename = "1")]
    Tabular(
        // Matching row inputs for a tabular query
        Vec<MatchingRowInput>,
        // The corresponding revelation input
        RevelationInput,
    ),

    /// Aggregation batching queries, next step is Revelation
    #[serde(rename = "2")]
    Aggregation(AggregationInput),

    /// Revelation step, we only handle aggregation revelation for now, next step is Groth16
    #[serde(rename = "3")]
    Revelation(RevelationInput),
}

/// Matching row input for a tabular query
#[derive(Clone, Dbg, PartialEq, Deserialize, Serialize)]
pub struct MatchingRowInput {
    /// Proof key of this row proof
    pub proof_key: ProofKey,
    /// Collumn cells info
    pub column_cells: RowCells,
    /// The placeholders
    pub placeholders: PlaceHolderLgn,
    /// Flag to identify if it's a leaf
    pub is_leaf: bool,
}

/// Input of an aggregation (batching) query
#[derive(Dbg, Clone, Deserialize, Serialize)]
pub struct AggregationInput {
    /// Proof key of this aggregation proof
    pub proof_key: ProofKey,
    /// Different proof inputs of an aggregation query
    pub input_kind: ProofInputKind,
}

/// Different proof inputs of an aggregation (batching) query
#[derive(Clone, Dbg, Deserialize, Serialize)]
pub enum ProofInputKind {
    /// Rows chunk input
    #[serde(rename = "1")]
    RowsChunk(RowsChunkInput),

    /// Chunk aggregation input
    #[serde(rename = "2")]
    ChunkAggregation(ChunkAggregationInput),

    /// Non existence input
    #[serde(rename = "3")]
    NonExistence(Box<NonExistenceInput>),
}

/// Handling a matching row proof, it could contain a proof key or the proof data.
#[derive(Clone, Dbg, Serialize, Deserialize)]
pub struct HydratableMatchingRow {
    pub proof: Hydratable<ProofKey>,
    pub path: RowPath,
    pub result: Vec<U256>,
}

impl HydratableMatchingRow {
    pub fn into_matching_row(self) -> MatchingRow {
        MatchingRow::new(self.proof.clone_proof(), self.path, self.result)
    }
}

/// Either a `Dehydrated` variant containing a key to a stored proof, or a
/// `Hydrated` containing the proof itself.
#[derive(Clone, Serialize, Deserialize)]
pub enum Hydratable<K: Clone + std::fmt::Debug> {
    Dehydrated(K),
    Hydrated(Arc<Vec<u8>>),
}

impl<T: Clone + std::fmt::Debug> std::fmt::Debug for Hydratable<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Hydratable::Dehydrated(k) => {
                write!(f, "dehydrated: {k:?}")
            },
            Hydratable::Hydrated(_) => {
                write!(f, "hydrated")
            },
        }
    }
}

impl<K: Clone + std::fmt::Debug> Hydratable<K> {
    /// Wrap a proof key into a `Dehydrated` variant.
    pub fn new(k: K) -> Self {
        Hydratable::Dehydrated(k)
    }

    /// Consume a `Hydrated` variant into its embedded proof; panic if it is
    /// not hydrated.
    pub fn proof(&self) -> Arc<Vec<u8>> {
        match self {
            Hydratable::Dehydrated(_) => unreachable!(),
            Hydratable::Hydrated(proof) => proof.clone(),
        }
    }

    /// Consume a `Hydrated` variant into its embedded proof; panic if it is
    /// not hydrated.
    pub fn clone_proof(&self) -> Vec<u8> {
        match self {
            Hydratable::Dehydrated(_) => unreachable!(),
            Hydratable::Hydrated(proof) => proof.clone().to_vec(),
        }
    }

    /// Convert a `Dehydrated` variant into its embedded key; panic if it is
    /// not hydrated.
    pub fn key(&self) -> K {
        match self {
            Hydratable::Dehydrated(k) => k.clone(),
            Hydratable::Hydrated(_) => unreachable!(),
        }
    }

    /// Hydrates a `Dehydrated` variant; panic if it is already hydrated.
    pub fn hydrate(
        &mut self,
        proof: Vec<u8>,
    ) {
        assert!(matches!(self, Hydratable::Dehydrated(_)));
        *self = Hydratable::Hydrated(Arc::new(proof))
    }
}

/// Revelation input
#[derive(Clone, Dbg, Deserialize, Serialize)]
pub enum RevelationInput {
    /// Input for an aggregation query
    Aggregated {
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
    /// Input for a tabular query
    Tabular {
        placeholders: PlaceHolderLgn,
        indexing_proof: Hydratable<db_keys::ProofKey>,
        matching_rows: Vec<HydratableMatchingRow>,
        column_ids: ColumnIDs,
        limit: u32,
        offset: u32,
    },
}

/// Non existence input of an aggregation query
#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct NonExistenceInput {
    pub index_path: TreePathInputs,

    pub column_ids: ColumnIDs,

    pub placeholders: PlaceHolderLgn,
}

impl From<&WorkerTask> for ProofKey {
    fn from(task: &WorkerTask) -> Self {
        match &task.task_type {
            WorkerTaskType::Query(qr) => qr.proof_key.clone(),
        }
    }
}

/// Rows chunk input of an aggregation query
#[derive(Clone, PartialEq, Dbg, Deserialize, Serialize)]
pub struct RowsChunkInput {
    pub rows: Vec<RowInput>,

    pub placeholders: PlaceHolderLgn,
}

/// Chunk aggregation input of an aggregation query
#[derive(Clone, Dbg, Deserialize, Serialize)]
pub struct ChunkAggregationInput {
    pub child_proofs: Vec<Hydratable<ProofKey>>,
}
