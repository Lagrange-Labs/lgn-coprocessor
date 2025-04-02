use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::query::tasks::Hydratable;
use crate::types::v1::query;

pub mod keys;

#[derive(Clone, Serialize, Deserialize, Dbg)]
pub struct WorkerTask {
    /// Chain ID
    pub chain_id: u64,

    /// The final proof
    #[dbg(skip)]
    pub revelation_proof: Hydratable<query::keys::ProofKey>,
}

impl WorkerTask {
    #[must_use]
    pub fn new(
        chain_id: u64,
        revelation_proof_location: query::keys::ProofKey,
    ) -> Self {
        Self {
            chain_id,
            revelation_proof: Hydratable::new(revelation_proof_location),
        }
    }
}
