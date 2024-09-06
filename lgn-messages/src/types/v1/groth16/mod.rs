use derive_debug_plus::Dbg;
use serde_derive::{Deserialize, Serialize};

pub mod keys;

/// Groth16 routing domain
pub const ROUTING_DOMAIN: &str = "sg";

#[derive(Clone, PartialEq, Serialize, Deserialize, Dbg)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// The final proof
    #[dbg(skip)]
    pub aggregated_result: Vec<u8>,
}

impl WorkerTask {
    #[must_use]
    pub fn new(chain_id: u64) -> Self {
        Self {
            chain_id,
            aggregated_result: vec![],
        }
    }
}
