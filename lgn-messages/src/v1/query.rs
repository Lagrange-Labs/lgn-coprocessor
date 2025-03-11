use std::collections::HashMap;

use alloy_primitives::U256;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use verifiable_db::api::QueryCircuitInput;
use verifiable_db::api::QueryParameters;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::computational_hash_ids::PlaceholderIdentifier;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::RowCells;
use verifiable_db::revelation::api::MatchingRow;
use verifiable_db::revelation::RowPath;

use crate::Proof;

/// Maximum number of chunks that can be aggregated in a single proof of batching query
/// We must use the same value of this constant for both DQ and Worker.
pub const NUM_CHUNKS: usize = 66;
/// Maximum number of rows that can be proven in a single proof of batching query
/// We must use the same value of this constant for both DQ and Worker.
pub const NUM_ROWS: usize = 100;

pub const ROW_TREE_MAX_DEPTH: usize = 25;
pub const INDEX_TREE_MAX_DEPTH: usize = 26;
pub const MAX_NUM_RESULT_OPS: usize = 20;
pub const MAX_NUM_RESULTS: usize = 10;
pub const MAX_NUM_OUTPUTS: usize = 5;
pub const MAX_NUM_ITEMS_PER_OUTPUT: usize = 5;
pub const MAX_NUM_PLACEHOLDERS: usize = 5;
pub const MAX_NUM_COLUMNS: usize = 20;
pub const MAX_NUM_PREDICATE_OPS: usize = 20;

pub type ConcreteQueryCircuitInput = QueryCircuitInput<
    NUM_CHUNKS,
    NUM_ROWS,
    ROW_TREE_MAX_DEPTH,
    INDEX_TREE_MAX_DEPTH,
    MAX_NUM_COLUMNS,
    MAX_NUM_PREDICATE_OPS,
    MAX_NUM_PREDICATE_OPS,
    MAX_NUM_OUTPUTS,
    MAX_NUM_ITEMS_PER_OUTPUT,
    MAX_NUM_PLACEHOLDERS,
>;

pub type ConcreteQueryParameters = QueryParameters<
    NUM_CHUNKS,
    NUM_ROWS,
    ROW_TREE_MAX_DEPTH,
    INDEX_TREE_MAX_DEPTH,
    MAX_NUM_COLUMNS,
    MAX_NUM_PREDICATE_OPS,
    MAX_NUM_RESULT_OPS,
    MAX_NUM_OUTPUTS,
    MAX_NUM_ITEMS_PER_OUTPUT,
    MAX_NUM_PLACEHOLDERS,
>;

#[derive(Deserialize, Serialize)]
pub enum QueryTask {
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

#[derive(PartialEq, Deserialize, Serialize)]
pub struct PlaceHolderLgn(HashMap<String, U256>);

impl From<PlaceHolderLgn> for Placeholders {
    fn from(place_holder: PlaceHolderLgn) -> Self {
        (&place_holder).into()
    }
}

impl From<Placeholders> for PlaceHolderLgn {
    fn from(place_holder: Placeholders) -> Self {
        (&place_holder).into()
    }
}

impl From<&PlaceHolderLgn> for Placeholders {
    fn from(place_holder: &PlaceHolderLgn) -> Self {
        let min_block = place_holder.0.get("0").cloned().unwrap();
        let max_block = place_holder.0.get("1").cloned().unwrap();
        let mut placeholders = Placeholders::new_empty(min_block, max_block);

        for (k, v) in place_holder.0.iter() {
            if k != "0" && k != "1" {
                let index = k.parse::<usize>().unwrap();
                placeholders.insert(PlaceholderIdentifier::Generic(index - 1), *v);
            }
        }

        placeholders
    }
}

impl From<&Placeholders> for PlaceHolderLgn {
    fn from(place_holder: &Placeholders) -> Self {
        let min_block = place_holder
            .get(&PlaceholderIdentifier::MinQueryOnIdx1)
            .unwrap();
        let max_block = place_holder
            .get(&PlaceholderIdentifier::MaxQueryOnIdx1)
            .unwrap();
        let mut map = HashMap::new();
        map.insert(0.to_string(), min_block);
        map.insert(1.to_string(), max_block);

        for (k, v) in place_holder.0.iter() {
            if let PlaceholderIdentifier::Generic(i) = k {
                map.insert((*i + 1).to_string(), *v);
            }
        }

        PlaceHolderLgn(map)
    }
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
