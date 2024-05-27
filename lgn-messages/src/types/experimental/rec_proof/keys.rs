use crate::types::experimental::rec_proof::{
    Experiment, Index, Level, LogMaxCapacity, LogSubsetSize,
};
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

type QueryId = String;

/// Identifies proofs in the storage system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Public params and inputs
    PublicParams(Experiment, LogMaxCapacity),

    /// Inputs(data)
    Inputs(Experiment, LogMaxCapacity, LogSubsetSize),

    /// Compute proof key
    Compute(QueryId, Level, Index),
}

impl From<ProofKey> for Path {
    fn from(key: ProofKey) -> Self {
        let path_str = match &key {
            ProofKey::PublicParams(experiment, log_max_capacity) => {
                format!("recproof/{experiment:?}/public_params/{log_max_capacity}")
            }
            ProofKey::Inputs(experiment, log_max_capacity, log_subset_size) => {
                format!("recproof/{experiment:?}/inputs/{log_max_capacity}/{log_subset_size}")
            }
            ProofKey::Compute(query_id, level, index) => {
                format!("recproof/node/{query_id}/{level}-{index}")
            }
        };

        Path::from(path_str)
    }
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Path::from(self.clone()))
    }
}

impl From<ProofKey> for String {
    fn from(key: ProofKey) -> Self {
        Path::from(key).to_string()
    }
}
