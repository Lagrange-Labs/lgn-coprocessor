use crate::types::experimental::tx_trie::keys::ProofKey;
use serde_derive::{Deserialize, Serialize};
use std::ops::RangeInclusive;

pub mod block;

pub mod block_range;

pub mod keys;

pub const ROUTING_DOMAIN: &str = "tx_trie";

#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
pub struct WorkerTask {
    /// What stage of proof generation process is this task for: single block, blocks tree or evm.
    pub task_type: WorkerTaskType,

    /// What kind of query is this task for. For example: sum of gas fees.
    pub computation: Computation,
}

impl WorkerTask {
    /// Initializes a new worker task.
    ///
    /// # Arguments
    /// * `task_type` - What stage of proof generation process is this task for.
    /// * `computation` - What kind of query is this task for.
    #[must_use]
    pub fn new(task_type: WorkerTaskType, computation: Computation) -> Self {
        Self {
            task_type,
            computation,
        }
    }

    /// Initializes a new worker task for a transaction trie proof.
    ///
    /// # Arguments
    /// * `range` - Range of transactions to prove.
    /// * `block_nr` - Which block transactions are we proving.
    #[must_use]
    pub fn block_transaction_task(
        range: RangeInclusive<usize>,
        block_nr: u64,
        computation: Computation,
    ) -> Self {
        let transactions =
            block::Transactions::new(block_nr, *range.start() as u64, *range.end() as u64);
        let proof_kind = WorkerTaskType::BlockProof(block::ProofKind::Transactions(transactions));
        WorkerTask::new(proof_kind, computation)
    }

    /// Initializes a new worker task for transaction trie node proof.
    ///
    /// # Arguments
    /// * `block_nr` - Which block transactions are we proving.
    /// * `node_id` - Indicates which node we prove in the trie.
    /// * `data_uris` - where to receive child nodes proofs from.
    #[must_use]
    pub fn block_intermediate_task(
        block_nr: u64,
        node_id: String,
        computation: Computation,
        data_uris: Vec<ProofKey>,
    ) -> Self {
        let proof_kind = WorkerTaskType::BlockProof(block::ProofKind::Intermediate(
            block::Intermediate::new(block_nr, node_id, data_uris),
        ));
        WorkerTask::new(proof_kind, computation)
    }

    /// Initializes a new worker task for a range of blocks proof.
    ///
    /// # Arguments
    /// * `computation` - What kind of query is this task for.
    /// * `data_uris` - where to receive child proofs from.
    #[must_use]
    pub fn block_range_task(computation: Computation, data_uris: Vec<ProofKey>) -> Self {
        let proof_kind = WorkerTaskType::BlocksRangeProof(block_range::ProofKind::Blocks(
            block_range::Blocks::new(data_uris),
        ));
        WorkerTask::new(proof_kind, computation)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WorkerReply {
    /// Query id this reply is for.
    pub query_id: String,

    /// Task id this reply is for.
    pub task_id: String,

    /// URI where the proof is stored.
    pub data_uri: ProofKey,
}

impl WorkerReply {
    /// Initializes a new worker reply.
    ///
    /// # Arguments
    /// * `query_id` - Query id this reply is for.
    /// * `task_id` - Task id this reply is for.
    /// * `data_uri` - URI where the proof is stored.
    #[must_use]
    pub fn new(query_id: String, task_id: String, data_uri: ProofKey) -> Self {
        Self {
            query_id,
            task_id,
            data_uri,
        }
    }
}

// See: https://github.com/serde-rs/serde/issues/745
#[derive(Clone, Debug, PartialEq, Hash, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    /// Task for a single block proof.
    #[serde(rename = "1")]
    BlockProof(block::ProofKind),

    /// Task for a range of blocks proof.
    #[serde(rename = "2")]
    BlocksRangeProof(block_range::ProofKind),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Computation {
    SumOfGasFees(SumOfGasFees),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SumOfGasFees {
    pub dest_address: String,
}

impl Computation {
    /// Uniquely identifies computation.
    #[must_use]
    pub fn id(&self) -> String {
        match self {
            Computation::SumOfGasFees(computation) => {
                format!("sum_of_gas_fees_{}", computation.dest_address)
            }
        }
    }
}
