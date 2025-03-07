use anyhow::bail;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::TaskType;

use crate::dummy_utils::dummy_proof;
use crate::provers::LgnProver;

const PROOF_SIZE: usize = 32;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct Groth16DummyProver;

impl LgnProver for Groth16DummyProver {
    fn run(
        &self,
        envelope: lgn_messages::types::MessageEnvelope,
    ) -> anyhow::Result<lgn_messages::types::MessageReplyEnvelope> {
        let task_id = envelope.task_id.clone();

        match envelope.task() {
            TaskType::V1Preprocessing(..) => {
                bail!("Groth16DummyProver: unsupported task type. task_type: V1Preprocessing task_id: {}", task_id)
            },
            TaskType::V1Query(..) => {
                panic!(
                    "Groth16DummyProver: unsupported task type. task_type: V1Query task_id: {}",
                    task_id
                )
            },
            TaskType::V1Groth16(_revelation_proof) => {
                Ok(MessageReplyEnvelope::new(task_id, dummy_proof(PROOF_SIZE)))
            },
        }
    }
}
