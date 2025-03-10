use alloy_primitives::U256;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::RowCells;
use verifiable_db::revelation::api::MatchingRow;
use verifiable_db::revelation::RowPath;

use super::ConcreteQueryCircuitInput;
use crate::types::v1::query::PlaceHolderLgn;
use crate::Proof;

/// Query input for a proving task
#[derive(Deserialize, Serialize)]
pub struct QueryInput {
    /// Query step info
    pub query_step: QueryStep,
}

/// Query step info
#[derive(Deserialize, Serialize)]
pub enum QueryStep {
    /// Inputs for a verifiable db circuit.
    ///
    /// This corresponds to a single circuit invocation.
    QueryCircuitInput(Box<ConcreteQueryCircuitInput>),

    /// Batched proof for tabular revelation
    BatchedTabular {
        rows_inputs: Vec<MatchingRowInput>,
        placeholders: PlaceHolderLgn,
        indexing_proof: Proof,
        matching_rows: Vec<HydratableMatchingRow>,
        column_ids: ColumnIDs,
        limit: u32,
        offset: u32,
        pis: Vec<u8>,
    },
}

/// Matching row input for a tabular query
#[derive(PartialEq, Deserialize, Serialize)]
pub struct MatchingRowInput {
    /// Collumn cells info
    pub column_cells: RowCells,
    /// The placeholders
    pub placeholders: PlaceHolderLgn,
    /// Flag to identify if it's a leaf
    pub is_leaf: bool,
}

/// Handling a matching row proof, it could contain a proof key or the proof data.
#[derive(Serialize, Deserialize)]
pub struct HydratableMatchingRow {
    pub path: RowPath,
    pub result: Vec<U256>,
}

impl HydratableMatchingRow {
    pub fn hydrate(
        self,
        proof: Vec<u8>,
    ) -> MatchingRow {
        MatchingRow::new(proof, self.path, self.result)
    }
}
