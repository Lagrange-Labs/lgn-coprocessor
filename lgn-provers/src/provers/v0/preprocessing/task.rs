use anyhow::Context;
use metrics::{counter, histogram};

use crate::provers::v0::preprocessing::prover::StorageProver;
use crate::provers::LgnProver;
use ethers::types::H256;
use lgn_messages::types::v0::preprocessing::keys::ProofKey;
use lgn_messages::types::v0::preprocessing::{
    MptData, StateDbData, StorageDbData, WorkerReply, WorkerTask, WorkerTaskType,
};
use lgn_messages::types::{MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType};
use std::time::Instant;
use tracing::debug;

impl<P: StorageProver> LgnProver for Preprocessing<P> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        self.run_inner(envelope)
    }
}

pub struct Preprocessing<P> {
    prover: P,
}

impl<P: StorageProver> Preprocessing<P> {
    pub(crate) fn new(prover: P) -> Self {
        Self { prover }
    }

    pub(crate) fn run_inner(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        debug!("Starting preprocessing task runner");
        if let TaskType::StoragePreprocess(task @ WorkerTask { block_nr, .. }) = envelope.inner() {
            let reply = self.process_task(*block_nr, task)?;
            let reply = ReplyType::StoragePreprocess(reply);
            let reply = MessageReplyEnvelope::new(envelope.query_id, envelope.task_id, reply);
            Ok(reply)
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }

    fn process_task(&mut self, block_nr: u64, task: &WorkerTask) -> anyhow::Result<WorkerReply> {
        debug!(?task, "Processing task");
        counter!("zkmr_worker_task_counter").increment(1);

        let maybe_proof = match &task.task_type {
            WorkerTaskType::Mpt(data) => match data {
                MptData::Leaf(data) => {
                    debug!(block_nr, ?data, "PROVING MPT LEAF");
                    let ts = Instant::now();

                    let key =
                        ProofKey::MptInclusion(block_nr, data.contract, encode_hash(data.hash))
                            .to_string();
                    let proof = self.prover.prove_mpt_leaf(data).unwrap();

                    histogram!("proving_latency", "proof_type" => "mpt_leaf")
                        .record(ts.elapsed().as_secs_f64());
                    debug!("Storing proof for leaf: {:?}", key);
                    Some((key, proof))
                }
                MptData::Branch(input) => {
                    debug!(block_nr, ?input, "PROVING MPT BRANCH");
                    let ts = Instant::now();

                    let key =
                        ProofKey::MptInclusion(block_nr, input.contract, encode_hash(input.hash))
                            .to_string();
                    let proof = self
                        .prover
                        .prove_mpt_branch(input)
                        .context("running prove_mpt_branch")?;

                    histogram!("proving_latency", "proof_type" => "mpt_intermediate")
                        .record(ts.elapsed().as_secs_f64());

                    debug!("Storing proof for branch: {}", key);
                    Some((key, proof))
                }
            },
            WorkerTaskType::StorageDb(input) => match input {
                StorageDbData::Leaf(leaf) => {
                    debug!(block_nr, ?leaf, "PROVING STORAGE DB LEAF");
                    let ts = Instant::now();

                    let key =
                        ProofKey::StorageDb(block_nr, leaf.contract, leaf.position).to_string();
                    let proof = self.prover.prove_storage_db_leaf(leaf.clone()).unwrap();

                    histogram!("proving_latency", "proof_type" => "storage_leaf")
                        .record(ts.elapsed().as_secs_f64());

                    Some((key, proof))
                }
                StorageDbData::Branch(branch) => {
                    debug!(block_nr, ?branch, "PROVING STORAGE DB BRANCH");
                    let ts = Instant::now();

                    let key =
                        ProofKey::StorageDb(block_nr, branch.contract, branch.position).to_string();
                    let proof = self
                        .prover
                        .prove_storage_db_branch(
                            branch.left_child_proof.to_vec(),
                            branch.right_child_proof.to_vec(),
                        )
                        .unwrap();

                    histogram!("proving_latency", "proof_type" => "storage_intermediate")
                        .record(ts.elapsed().as_secs_f64());

                    Some((key, proof))
                }
            },
            WorkerTaskType::LengthExtract(data) => {
                debug!(block_nr, ?data, "PROVING LENGTH SLOT");
                let ts = Instant::now();

                let key = ProofKey::LengthSlot(block_nr, data.contract).to_string();
                let proof = self
                    .prover
                    .prove_length_extract(data.clone())
                    .context("running prove_length_extract")?;

                histogram!("proving_latency", "proof_type" => "length_extract")
                    .record(ts.elapsed().as_secs_f64());

                debug!("Storing proof for length slot: {key}");
                Some((key, proof))
            }
            WorkerTaskType::LengthMatch(data) => {
                debug!(block_nr, ?data, "PROVING LENGTH MATCH");

                let ts = Instant::now();

                let key = ProofKey::Bridge(block_nr, data.contract).to_string();
                let proof = self
                    .prover
                    .prove_length_match(&data.mapping_proof, &data.length_extract_proof)
                    .context("runnning prove_length_match")?;

                histogram!("proving_latency", "proof_type" => "length_match")
                    .record(ts.elapsed().as_secs_f64());

                debug!("Storing proof for bridge: {}", key);
                Some((key, proof))
            }
            WorkerTaskType::Equivalence(data) => {
                debug!(block_nr, ?data, "PROVING EQUIVALENCE");
                let ts = Instant::now();

                let key = ProofKey::Equivalence(block_nr, data.contract).to_string();
                let proof = self
                    .prover
                    .prove_equivalence(
                        data.storage_proof.to_vec(),
                        data.length_match_proof.to_vec(),
                    )
                    .context("runnning prove_equivalence")?;

                histogram!("proving_latency", "proof_type" => "equivalence")
                    .record(ts.elapsed().as_secs_f64());

                debug!("Storing proof for equivalence: {}", key);
                Some((key, proof))
            }
            WorkerTaskType::BlockLinking(data) => {
                debug!("Proving block linking: {:?}", block_nr);

                let ts = Instant::now();
                let key = ProofKey::BlockLinking(block_nr).to_string();
                let proof = self.prover.prove_block_number_linking(data).unwrap();

                histogram!("proving_latency", "proof_type" => "block_linking")
                    .record(ts.elapsed().as_secs_f64());

                debug!("Storing proof for block header linking: {}", key);
                Some((key, proof))
            }
            WorkerTaskType::StateDb(data) => match data {
                StateDbData::Leaf(leaf) => {
                    debug!("Proving state db leaf: {:?}", leaf);
                    let ts = Instant::now();

                    let key = ProofKey::State(block_nr, leaf.position).to_string();
                    let proof = self
                        .prover
                        .prove_state_db_leaf(leaf.block_linking_proof.to_vec())
                        .unwrap();
                    histogram!("proving_latency", "proof_type" => "state_db_leaf")
                        .record(ts.elapsed().as_secs_f64());

                    debug!("Storing proof for leaf: {}", key);
                    Some((key, proof))
                }
                StateDbData::Branch(branch) => {
                    debug!("PROVING STATE DB BRANCH: {:?}", branch);
                    let ts = Instant::now();

                    let key = ProofKey::State(block_nr, branch.position).to_string();
                    let proof = self
                        .prover
                        .prove_state_db_branch(
                            branch.left_proof.to_vec(),
                            branch.right_proof.to_vec(),
                        )
                        .unwrap();
                    histogram!("proving_latency", "proof_type" => "state_db_branch")
                        .record(ts.elapsed().as_secs_f64());

                    debug!("Storing proof for branch: {}", key);
                    Some((key, proof))
                }
            },
            WorkerTaskType::BlocksDb(data) => {
                let ts = Instant::now();

                let key = ProofKey::BlocksDb(block_nr, data.leaf_index).to_string();
                let proof = if data.leaf_index == 0 {
                    debug!("Proving blocks db first block: {:?}", data.leaf_index);
                    self.prover
                        .prove_blocks_db_first(data.clone())
                        .context("running prove_blocks_db_first")?
                } else {
                    debug!("Proving blocks db subsequent block: {:?}", data.leaf_index);
                    self.prover
                        .prove_blocks_db_subsequent(data.clone())
                        .context("running prove_blocks_db_subsequent")?
                };
                histogram!("proving_latency", "proof_type" => "blocks_db")
                    .record(ts.elapsed().as_secs_f64());

                debug!("Storing proof for blocks db: {:?}", key);
                Some((key, proof))
            }
        };

        Ok(WorkerReply::new(task.chain_id, task.block_nr, maybe_proof))
    }
}

fn encode_hash(hash: H256) -> String {
    hex::encode(hash)[..8].to_string()
}
