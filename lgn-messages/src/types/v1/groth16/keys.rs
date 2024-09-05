use crate::types::v1::query::keys::KEYS_QUERIES_PREFIX;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

pub type QueryId = String;

/// Where to store the Groth16 proof
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub struct ProofKey(pub QueryId);

impl Display for ProofKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let query_id = &self.0;
        write!(f, "{KEYS_QUERIES_PREFIX}/{query_id}/groth16")
    }
}

impl From<ProofKey> for Path {
    fn from(key: ProofKey) -> Self {
        Path::from(key.to_string())
    }
}

/// List the all asset keys.
pub const ALL_ASSET_KEYS: [AssetKey; 5] = [
    AssetKey::Circuit,
    AssetKey::R1CS,
    AssetKey::PK,
    AssetKey::VK,
    AssetKey::VerifierContract,
];

/// Where to store the Groth16 asset files
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum AssetKey {
    /// Asset file `circuit.bin`
    Circuit,

    /// Asset file `r1cs.bin`
    R1CS,

    /// Asset file `pk.bin`
    PK,

    /// Asset file `vk.bin`
    VK,

    /// Asset file `verifier.sol`
    VerifierContract,
}

impl AssetKey {
    /// Return the asset filename.
    #[must_use]
    pub fn filename(&self) -> &str {
        match self {
            AssetKey::Circuit => "circuit.bin",
            AssetKey::R1CS => "r1cs.bin",
            AssetKey::PK => "pk.bin",
            AssetKey::VK => "vk.bin",
            AssetKey::VerifierContract => "verifier.sol",
        }
    }
}
