use derive_debug_plus::Dbg;
use ethers::types::Address;
use serde_derive::{Deserialize, Serialize};

pub mod keys;

/// Groth16 routing domain
pub const ROUTING_DOMAIN: &str = "sg";

#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    /// Generate the Groth16 proof.
    #[serde(rename = "1")]
    Prove,
}

#[derive(Clone, Serialize, Deserialize, Dbg)]
pub struct WorkerTask {
    /// Which contract this task is for.
    pub contract: Address,

    /// Chain ID
    pub chain_id: u64,

    /// Task type to handle.
    pub task_type: WorkerTaskType,

    /// The final proof
    #[dbg(skip)]
    pub aggregated_result: Vec<u8>,
}

impl WorkerTask {
    #[must_use]
    pub fn new(chain_id: u64, contract: Address, task_type: WorkerTaskType) -> Self {
        Self {
            contract,
            chain_id,
            task_type,
            aggregated_result: Vec::default(),
        }
    }
}
