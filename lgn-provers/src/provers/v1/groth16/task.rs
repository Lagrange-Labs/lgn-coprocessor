use std::time::Instant;

use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::v1::groth16::keys::ProofKey;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;
use tracing::debug;
use tracing::info;

use super::euclid_prover::Groth16Prover;
use crate::provers::LgnProver;

impl LgnProver for Groth16Prover {
    fn run(
        &self,
        envelope: MessageEnvelope,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Groth16(task) = envelope.inner() {
            let proof = self.generate_proof(
                &query_id,
                &task_id,
                task.revelation_proof.proof().as_slice(),
            )?;
            let reply = WorkerReply::new(task.chain_id, Some(proof), ProofCategory::Querying);
            let reply_type = ReplyType::V1Groth16(reply);
            let reply_envelope = MessageReplyEnvelope::new(query_id, task_id, reply_type);
            Ok(reply_envelope)
        } else {
            bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }
}

impl Groth16Prover {
    /// Generate the Groth proof.
    fn generate_proof(
        &self,
        query_id: &str,
        task_id: &str,
        revelation: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>)> {
        // Generate the Groth16 proof.
        let now = Instant::now();
        let key = ProofKey(query_id.to_string()).to_string();
        let proof = self.prove(revelation).with_context(|| {
            format!(
                " Failed to generate the Groth16 proof: query_id = {query_id}, task_id = {task_id}"
            )
        })?;
        debug!("Finish generating the Groth16 proof: query_id = {query_id}, task_id = {task_id}",);

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "groth16",
            "proof generation time: {:?}",
            now.elapsed()
        );

        Ok((key, proof))
    }
}
