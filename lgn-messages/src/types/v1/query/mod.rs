use std::collections::HashMap;

use alloy_primitives::U256;
use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use verifiable_db::query::computational_hash_ids::PlaceholderIdentifier;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;

use crate::types::v1::query::tasks::QueryInput;

pub mod keys;
pub mod tasks;

pub const ROUTING_DOMAIN: &str = "sc";

/// Maximum number of chunks that can be aggregated in a single proof of batching query
/// We must use the same value of this constant for both DQ and Worker.
pub const NUM_CHUNKS: usize = 66;
/// Maximum number of rows that can be proven in a single proof of batching query
/// We must use the same value of this constant for both DQ and Worker.
pub const NUM_ROWS: usize = 100;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(
        chain_id: u64,
        task_type: WorkerTaskType,
    ) -> Self {
        Self {
            chain_id,
            task_type,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Query(QueryInput),
}

#[derive(Dbg, Clone, PartialEq, Deserialize, Serialize)]
pub struct PlaceHolderLgn(HashMap<String, U256>);

impl From<PlaceHolderLgn> for Placeholders {
    fn from(ph: PlaceHolderLgn) -> Self {
        let min_block =
            ph.0.get("0")
                .cloned()
                .unwrap();
        let max_block =
            ph.0.get("1")
                .cloned()
                .unwrap();
        let mut placeholders = Placeholders::new_empty(
            min_block,
            max_block,
        );

        for (k, v) in
            ph.0.into_iter()
        {
            if k != "0" && k != "1" {
                let index = k
                    .parse::<usize>()
                    .unwrap();
                placeholders.insert(
                    PlaceholderIdentifier::Generic(index - 1),
                    v,
                );
            }
        }

        placeholders
    }
}

impl From<Placeholders> for PlaceHolderLgn {
    fn from(ph: Placeholders) -> Self {
        let min_block = ph
            .get(&PlaceholderIdentifier::MinQueryOnIdx1)
            .unwrap();
        let max_block = ph
            .get(&PlaceholderIdentifier::MaxQueryOnIdx1)
            .unwrap();
        let mut map = HashMap::new();
        map.insert(
            0.to_string(),
            min_block,
        );
        map.insert(
            1.to_string(),
            max_block,
        );

        for (k, v) in
            ph.0.iter()
        {
            if let PlaceholderIdentifier::Generic(i) = k {
                map.insert(
                    (*i + 1).to_string(),
                    *v,
                );
            }
        }

        PlaceHolderLgn(map)
    }
}
