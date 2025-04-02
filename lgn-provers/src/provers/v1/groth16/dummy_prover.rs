use anyhow::bail;
use lgn_messages::types::v1::groth16::keys::ProofKey;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;

use crate::dummy_utils::dummy_proof;
use crate::provers::LgnProver;

const PROOF_SIZE: usize = 32;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl LgnProver for DummyProver {
    fn run(
        &self,
        envelope: &MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Groth16(task) = envelope.inner() {
            let key = ProofKey(query_id.to_string()).to_string();
            let proof = dummy_proof(PROOF_SIZE);
            let reply =
                WorkerReply::new(task.chain_id, Some((key, proof)), ProofCategory::Querying);
            let reply_type = ReplyType::V1Groth16(reply);
            let reply_envelope = MessageReplyEnvelope::new(query_id, task_id, reply_type);
            Ok(reply_envelope)
        } else {
            bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }
}
