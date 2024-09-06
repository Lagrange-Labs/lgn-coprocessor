use super::prover::Prover;
use anyhow::{bail, Context};

use crate::provers::LgnProver;
use lgn_messages::types::v1::groth16::keys::ProofKey;
use lgn_messages::types::v1::groth16::WorkerTask;
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ProofCategory, ReplyType, TaskType, WorkerReply,
};
use std::time::Instant;
use tracing::{debug, info};

impl<GP: Prover> LgnProver<TaskType, ReplyType> for Groth16<GP> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        self.run_inner(envelope)
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

    pub(crate) fn run_inner(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Groth16(task) = envelope.inner() {
            let reply = self.process_task(query_id.clone(), task_id.clone(), task)?;
            let reply_type = ReplyType::V1Groth16(reply);
            let reply_envelope = MessageReplyEnvelope::new(query_id, task_id, reply_type);
            Ok(reply_envelope)
        } else {
            bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }

    fn process_task(
        &mut self,
        query_id: String,
        task_id: String,
        task: &WorkerTask,
    ) -> anyhow::Result<WorkerReply> {
        let proof = self.generate_proof(&query_id, &task_id, &task.revelation_proof)?;
        Ok(WorkerReply::new(
            task.chain_id,
            Some(proof),
            ProofCategory::Querying,
        ))
    }

    /// Generate the Groth proof.
    fn generate_proof(
        &mut self,
        query_id: &str,
        task_id: &str,
        revelation: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>)> {
        // Generate the Groth16 proof.
        let now = Instant::now();
        let key = ProofKey(query_id.to_string()).to_string();
        let proof = self.prover.prove(revelation).with_context(|| {
            format!(
                " Failed to generate the Groth16 proof: query_id = {query_id}, task_id = {task_id}"
            )
        })?;
        info!("Finish generating the Groth16 proof: query_id = {query_id}, task_id = {task_id}",);

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "groth16",
            "Groth16 proof generation time: {:?}",
            now.elapsed()
        );

        Ok((key, proof))
    }
}