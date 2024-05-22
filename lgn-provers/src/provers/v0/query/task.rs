use anyhow::Context;
use lgn_messages::types::v0::query::keys::ProofKey;
use lgn_messages::types::v0::query::{Query2BlockData, WorkerReply, WorkerTask, WorkerTaskType};
use lgn_messages::types::{MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType};

use crate::provers::v0::query::prover::QueryProver;
use crate::provers::LgnProver;

impl<P: QueryProver> LgnProver for Query<P> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        self.run_inner(envelope)
    }
}

pub struct Query<P> {
    prover: P,
}

impl<P: QueryProver> Query<P> {
    pub(crate) fn new(prover: P) -> Self {
        Self { prover }
    }

    pub(crate) fn run_inner(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::StorageQuery(task) = envelope.inner() {
            let reply = self.process_task(query_id.clone(), task_id.clone(), task)?;
            let reply_type = ReplyType::StorageQuery(reply);
            let reply_envelope = MessageReplyEnvelope::new(query_id, task_id, reply_type);
            Ok(reply_envelope)
        } else {
            anyhow::bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }

    fn process_task(
        &mut self,
        query_id: String,
        task_id: String,
        task: &WorkerTask,
    ) -> anyhow::Result<WorkerReply> {
        let maybe_proof = match &task.task_type {
            WorkerTaskType::StorageEntry(data) => {
                let key = ProofKey::StorageEntry(
                    query_id.clone().clone(),
                    data.block_nr,
                    task.contract,
                    data.position,
                )
                .to_string();
                let proof = self
                    .prover
                    .prove_storage_entry(data.inputs.clone())
                    .context("while running prove_storage_entry")?;

                Some((key, proof))
            }
            WorkerTaskType::StateEntry(data) => {
                let key =
                    ProofKey::StateDatabase(query_id.clone(), data.block_number, task.contract)
                        .to_string();
                let proof = self
                    .prover
                    .prove_state_db(data)
                    .context("while running prove_state_db")?;

                Some((key, proof))
            }
            WorkerTaskType::Aggregation(data) => match data {
                Query2BlockData::FullNode(ref input) => {
                    let key = ProofKey::Aggregation(query_id.clone(), data.position()).to_string();
                    let proof = self
                        .prover
                        .prove_block_full_node(
                            input.left_child_proof.as_ref(),
                            input.right_child_proof.as_ref(),
                        )
                        .context("while running prove_block_full_node")?;

                    Some((key, proof))
                }
                Query2BlockData::PartialNode(ref input) => {
                    let key = ProofKey::Aggregation(query_id.clone(), data.position()).to_string();
                    let proof = self
                        .prover
                        .prove_block_partial_node(input)
                        .context("while running prove_block_partial_node")?;

                    Some((key, proof))
                }
            },
            WorkerTaskType::Revelation(data) => {
                let key = ProofKey::Revelation(query_id.clone()).to_string();
                let proof = self
                    .prover
                    .prove_revelation(data)
                    .context("while running prove_revelation")?;

                Some((key, proof))
            }
        };

        Ok(WorkerReply::new(query_id, task_id, maybe_proof))
    }
}
