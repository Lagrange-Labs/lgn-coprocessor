use anyhow::bail;
use lgn_messages::types::v1::groth16::keys::ProofKey;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::RequestVersioned;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;

use super::euclid_prover::Groth16EuclidProver;
use crate::provers::LgnProver;

impl LgnProver for Groth16EuclidProver {
    fn run(
        &self,
        envelope: RequestVersioned,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id();
        let task_id = envelope.task_id();

        if let TaskType::V1Groth16(task) = envelope.inner() {
            let key = ProofKey(query_id.to_string()).to_string();
            let proof = self.prove(task.revelation_proof.proof().as_slice())?;

            let reply =
                WorkerReply::new(task.chain_id, Some((key, proof)), ProofCategory::Querying);
            let reply_type = ReplyType::V1Groth16(reply);
            let reply_envelope =
                MessageReplyEnvelope::new(query_id.to_owned(), task_id.to_owned(), reply_type);
            Ok(reply_envelope)
        } else {
            bail!("Unexpected task: {:?}", envelope);
        }
    }
}
