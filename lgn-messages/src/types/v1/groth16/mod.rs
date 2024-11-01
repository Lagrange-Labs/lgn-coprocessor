use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::query;

pub mod keys;

/// Groth16 routing domain
pub const ROUTING_DOMAIN: &str = "sg";

#[derive(Clone, PartialEq, Serialize, Deserialize, Dbg)]
pub struct WorkerTask
{
    /// Chain ID
    pub chain_id: u64,

    pub revelation_proof_location: query::keys::ProofKey,

    /// The final proof
    #[dbg(skip)]
    pub revelation_proof: Vec<u8>,
}

impl WorkerTask
{
    #[must_use]
    pub fn new(
        chain_id: u64,
        revelation_proof_location: query::keys::ProofKey,
    ) -> Self
    {
        Self {
            chain_id,
            revelation_proof_location,
            revelation_proof: vec![],
        }
    }
}
