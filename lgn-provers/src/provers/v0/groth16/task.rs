use super::prover::Prover;
use anyhow::{bail, Context};

use crate::provers::LgnProver;
use lgn_messages::types::v0::groth16::keys::ProofKey;
use lgn_messages::types::v0::groth16::WorkerTask;
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, WorkerReply,
};
use std::time::Instant;
use tracing::{debug, info};

impl<GP: Prover> LgnProver for Groth16<GP> {
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
        if let TaskType::StorageGroth16(task) = envelope.inner() {
            let reply = self.process_task(query_id.clone(), task_id.clone(), task)?;
            let reply_type = ReplyType::StorageGroth16(reply);
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
        let proof = self.generate_proof(&query_id, &task_id, &task.aggregated_result)?;
        Ok(WorkerReply::new(task.chain_id, Some(proof)))
    }

    /// Generate the Groth proof.
    fn generate_proof(
        &mut self,
        query_id: &str,
        task_id: &str,
        aggregated_proof: &[u8],
    ) -> anyhow::Result<(String, Vec<u8>)> {
        // Generate the Groth16 proof.
        let now = Instant::now();
        let key = ProofKey(query_id.to_string()).to_string();
        let proof = self.prover.prove(aggregated_proof).with_context(|| {
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

#[cfg(test)]
mod tests {
    use crate::provers::v0::groth16::create_prover;
    use groth16_framework::utils::{read_file, write_file};
    use std::path::Path;

    /// Test Groth16 initialize and generate the proof.
    ///
    /// It should be run in the `worker` and must make the followings before
    /// running this test:
    /// - Edit `config/gen-groth16-assets-conf.example.toml`, then generate the
    ///   Groth16 asset files and upload to S3 by:
    ///   cargo run --bin gen-groth16-assets -- --config ./config/gen-groth16-assets-conf.example.toml
    /// - Edit `config/worker-conf.example.toml` for initializing
    ///   `Groth16TaskRunner` in this test.
    /// - Save a query proof to file `worker/test_data/query.proof` for testing.
    #[ignore] // Ignore for long running time in CI.
    #[test]
    fn test_groth16_init_and_prove() {
        // Initialize the constants.
        let query_id = "q-100";
        let task_id = "t-100";

        // Create a test Groth16 task runner.

        let mut runner = create_prover(
            "url",
            "dir",
            "test",
            "circuit_file",
            "pk_file",
            "vk_file",
            true,
        )
        .unwrap();

        // Load the test query proof for generating the Groth16 proof later.
        let payload = read_file(Path::new("test_data").join("query.proof")).unwrap();

        // Initialize the Groth16 prover and generate the proof.
        let groth16_proof = runner.generate_proof(query_id, task_id, &payload).unwrap();

        // Download and save the Groth16 proof (for further verification).
        save_groth16_proof(&groth16_proof.1).unwrap();
    }

    /// Download and save the Groth16 proof. It could be verified with the
    /// generated verifier contract in mapreduce-plonky2.
    fn save_groth16_proof(groth16_proof: &[u8]) -> anyhow::Result<()> {
        // Save the generated Groth16 proof.
        write_file(Path::new("test_data").join("groth16.proof"), groth16_proof)

        // TODO: May verify the Groth16 proof off-chain here, it needs to add a
        // conversion function for combine Groth16 proofs in mapreduce-plonky2.
    }
}
