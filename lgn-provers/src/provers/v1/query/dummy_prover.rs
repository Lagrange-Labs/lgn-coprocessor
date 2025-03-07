use anyhow::bail;
use lgn_messages::types::v1::query::WorkerTaskType;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::TaskType;

use crate::dummy_utils::dummy_proof;
use crate::provers::LgnProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct QueryDummyProver;

impl LgnProver for QueryDummyProver {
    fn run(
        &self,
        envelope: lgn_messages::types::MessageEnvelope,
    ) -> anyhow::Result<lgn_messages::types::MessageReplyEnvelope> {
        let task_id = envelope.task_id.clone();

        match envelope.task() {
            TaskType::V1Preprocessing(..) => {
                bail!("QueryDummyProver: unsupported task type. task_type: V1Preprocessing task_id: {}", task_id)
            },
            TaskType::V1Query(WorkerTaskType::Query(..)) => {
                let proof = dummy_proof(PROOF_SIZE);
                Ok(MessageReplyEnvelope::new(task_id, proof))
            },
            TaskType::V1Groth16(..) => {
                bail!(
                    "QueryDummyProver: unsupported task type. task_type: V1Groth16 task_id: {}",
                    task_id,
                )
            },
        }
    }
}
