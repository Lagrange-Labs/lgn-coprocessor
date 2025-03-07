use anyhow::bail;
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

        if let TaskType::V1Query(..) = envelope.task {
            let proof = dummy_proof(PROOF_SIZE);
            Ok(MessageReplyEnvelope::new(task_id, proof))
        } else {
            bail!("Received unexpected task: {:?}", envelope)
        }
    }
}
