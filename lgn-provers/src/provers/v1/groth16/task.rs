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

use super::prover::Prover;
use crate::provers::LgnProver;

impl<GP: Prover> LgnProver<TaskType, ReplyType> for Groth16<GP> {
    fn run(
        &self,
        envelope: &MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        if let TaskType::V1Groth16(revelation_proof) = envelope.inner() {
            let now = Instant::now();
            let key = ProofKey(envelope.query_id.to_string()).to_string();
            let proof = self
                .prover
                .prove(revelation_proof.as_slice())
                .with_context(|| {
                    format!(
                        "Failed to generate the Groth16 proof: query_id = {}, task_id = {}",
                        envelope.query_id, envelope.task_id,
                    )
                })?;
            debug!(
                "Finish generating the Groth16 proof: query_id = {}, task_id = {}",
                envelope.query_id, envelope.task_id,
            );

            info!(
                time = now.elapsed().as_secs_f32(),
                proof_type = "groth16",
                "proof generation time: {:?}",
                now.elapsed()
            );

            let proof = (key, proof);
            let reply = WorkerReply::new(Some(proof), ProofCategory::Querying);

            let reply_type = ReplyType::V1Groth16(reply);
            let reply_envelope = MessageReplyEnvelope::new(
                envelope.query_id.clone(),
                envelope.task_id.clone(),
                reply_type,
            );
            Ok(reply_envelope)
        } else {
            bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }
}

pub struct Groth16<GP> {
    /// The Groth16 prover only initialized once
    prover: GP,
}

impl<GP: Prover> Groth16<GP> {
    pub(crate) fn new(prover: GP) -> Groth16<GP> {
        Self { prover }
    }
}
