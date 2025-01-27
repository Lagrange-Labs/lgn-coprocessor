use std::collections::hash_map::DefaultHasher;
use std::fmt::Display;
use std::hash::Hash;
use std::hash::Hasher;

use ethers::types::Transaction;
use ethers::utils::hex;
use ethers::utils::keccak256;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use crate::types::experimental::tx_trie::Computation;

type ComputationId = String;
type TxHash = String;
type IntermediateNodeHash = String;
type AggregationNodeHash = String;
type QueryId = String;

/// Indicates where proof is stored
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Transaction proof key with the transaction hash in hex
    Transaction(
        ComputationId,
        u64,
        TxHash,
    ),

    /// Intermediate proof key with the hash of intermediate node bytes in hex
    Intermediate(
        ComputationId,
        u64,
        IntermediateNodeHash,
    ),

    /// Header proof key with block number
    Block(
        ComputationId,
        u64,
    ),

    /// Header proof key with the inclusive range of header heights
    Aggregation(
        ComputationId,
        AggregationNodeHash,
    ),

    /// Result proof key with query id
    Result(QueryId),
}

impl ProofKey {
    /// Initializes a new proof key for a transaction proof.
    ///
    /// # Arguments
    /// * `computation` - Computation this proof key is for.
    /// * `tx` - Transaction this proof key is for.
    pub fn transaction(
        computation: &Computation,
        tx: &Transaction,
    ) -> Self {
        Self::Transaction(
            computation.id(),
            tx.block_number
                .expect("Expected block number")
                .as_u64(),
            hex::encode(
                tx.hash()
                    .as_bytes(),
            ),
        )
    }

    /// Initializes a new proof key for transaction trie intermediate node proof.
    ///
    /// # Arguments
    /// * `computation` - Computation this proof key is for.
    /// * `block_nr` - Block number this proof key is for.
    /// * `intermediate_node_bytes` - used to hex encode the intermediate node hash.
    pub fn intermediate(
        computation: &Computation,
        block_nr: u64,
        intermediate_node_bytes: impl AsRef<[u8]>,
    ) -> Self {
        Self::Intermediate(
            computation.id(),
            block_nr,
            hex::encode(keccak256(intermediate_node_bytes.as_ref())),
        )
    }

    /// Initializes a new proof key for a block proof.
    ///
    /// # Arguments
    /// * `computation` - Computation this proof key is for.
    /// * `block_nr` - Block number this proof key is for.
    #[must_use]
    pub fn block(
        computation: &Computation,
        block_nr: u64,
    ) -> Self {
        Self::Block(
            computation.id(),
            block_nr,
        )
    }

    /// Initializes a new proof key for an aggregation proof.
    ///
    /// # Arguments
    /// * `computation` - Computation this proof key is for.
    /// * `child_keys` - Locations of child proofs.
    #[must_use]
    pub fn aggregation(
        computation: &Computation,
        child_keys: Vec<ProofKey>,
    ) -> Self {
        let mut child_keys = child_keys;
        child_keys.sort();
        let data = child_keys
            .iter()
            .fold(
                Vec::new(),
                |mut acc, key| {
                    let hash = ProofKey::calculate_hash(key);
                    acc.extend_from_slice(&hash);
                    acc
                },
            );
        let node_hash = keccak256(data);
        Self::Aggregation(
            computation.id(),
            hex::encode(node_hash),
        )
    }

    #[must_use]
    pub fn result(query_id: &str) -> Self {
        Self::Result(query_id.to_string())
    }

    fn calculate_hash<T: Hash>(t: &T) -> Vec<u8> {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
            .to_be_bytes()
            .to_vec()
    }
}

impl From<ProofKey> for String {
    fn from(key: ProofKey) -> Self {
        key.to_string()
    }
}

impl Display for ProofKey {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ProofKey::Transaction(computation_id, block_nr, tx_hash) => {
                write!(
                    f,
                    "tx_{computation_id}_{block_nr}_{tx_hash}"
                )
            },
            ProofKey::Intermediate(computation_id, block_nr, intermediate_node_hash) => {
                write!(
                    f,
                    "intermediate_{computation_id}_{block_nr}_{intermediate_node_hash}"
                )
            },
            ProofKey::Block(computation_id, block_nr) => {
                write!(
                    f,
                    "block_{computation_id}_{block_nr}"
                )
            },
            ProofKey::Aggregation(computation_id, aggregation_node_hash) => {
                write!(
                    f,
                    "aggregation_{computation_id}_{aggregation_node_hash}"
                )
            },
            ProofKey::Result(query_id) => {
                write!(
                    f,
                    "result_{query_id}"
                )
            },
        }
    }
}
