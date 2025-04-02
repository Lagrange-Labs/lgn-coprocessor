use std::fmt::Display;

use object_store::path::Path;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::v1::query::keys::KEYS_QUERIES_PREFIX;

pub type QueryId = String;

/// Where to store the Groth16 proof
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct ProofKey(pub QueryId);

impl Display for ProofKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let query_id = &self.0;
        write!(f, "{KEYS_QUERIES_PREFIX}/{query_id}/groth16")
    }
}

impl From<ProofKey> for Path {
    fn from(key: ProofKey) -> Self {
        Path::from(key.to_string())
    }
}
