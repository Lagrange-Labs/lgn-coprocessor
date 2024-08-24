use crate::types::v1::query::tasks::QueryInput;
use serde_derive::{Deserialize, Serialize};

pub mod keys;
pub mod tasks;

pub(crate) const KEYS_QUERIES_PREFIX: &str = "V1_QUERIES";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// What we are proving.
    pub task_type: WorkerTaskType,
}

impl WorkerTask {
    #[must_use]
    pub fn new(chain_id: u64, task_type: WorkerTaskType) -> Self {
        Self {
            chain_id,
            task_type,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    #[serde(rename = "1")]
    Query(QueryInput),
}
