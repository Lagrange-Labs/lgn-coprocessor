pub mod keys;

use crate::types::KeyedPayload;
use derive_debug_plus::Dbg;
use ethers::types::Address;
use serde_derive::{Deserialize, Serialize};

/// Groth16 routing domain
pub const ROUTING_DOMAIN: &str = "sg";

#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    /// Generate the Groth16 proof.
    #[serde(rename = "2")]
    Prove,
}

#[derive(Clone, Serialize, Deserialize, Dbg)]
pub struct WorkerTask {
    /// Which contract this task is for.
    pub contract: Address,

    /// Task type to handle.
    pub task_type: WorkerTaskType,

    /// The final proof
    #[dbg(skip)]
    pub aggregated_result: Vec<u8>,
}

impl WorkerTask {
    #[must_use]
    pub fn new(contract: Address, task_type: WorkerTaskType) -> Self {
        Self {
            contract,
            task_type,
            aggregated_result: Vec::default(),
        }
    }
}

#[derive(Clone, Dbg, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WorkerReply {
    pub query_id: String,
    pub task_id: String,
    #[dbg(formatter = crate::types::kp_pretty)]
    pub proof: Option<KeyedPayload>,
}

impl WorkerReply {
    #[must_use]
    pub fn new(query_id: String, task_id: String, proof: Option<KeyedPayload>) -> Self {
        Self {
            query_id,
            task_id,
            proof,
        }
    }
}
