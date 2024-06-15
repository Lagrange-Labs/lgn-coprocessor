use crate::provers::v0::query::erc20::prover::QueryProver;
use crate::provers::LgnProver;
use lgn_messages::types::v0::query::erc20::keys::ProofKey;
use lgn_messages::types::v0::query::erc20::{
    BlocksDbData, StorageData, WorkerTask, WorkerTaskType,
};
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, WorkerReply,
};

pub struct Query<P> {
    prover: P,
}

impl<P: QueryProver> LgnProver for Query<P> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        self.run_inner(envelope)
    }
}

impl<P: QueryProver> Query<P> {
    pub(crate) fn new(prover: P) -> Self {
        Self { prover }
    }

    pub(crate) fn run_inner(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        if let TaskType::Erc20Query(task) = envelope.inner() {
            let reply = self.process_task(envelope.query_id.clone(), task)?;
            let reply_type = ReplyType::Erc20Query(reply);
            Ok(MessageReplyEnvelope::new(
                envelope.query_id,
                envelope.task_id,
                reply_type,
            ))
        } else {
            anyhow::bail!("Unexpected task type: {:?}", envelope.inner());
        }
    }

    fn process_task(&mut self, query_id: String, task: &WorkerTask) -> anyhow::Result<WorkerReply> {
        let proof = match &task.task_type {
            WorkerTaskType::StorageEntry(input) => match input {
                StorageData::StorageLeaf(data) => {
                    let proof = self.prover.prove_storage_leaf(task.contract, data)?;
                    let key = ProofKey::StorageEntry(query_id, data.block_number, data.position)
                        .to_string();
                    Some((key, proof))
                }
                StorageData::StorageBranch(data) => {
                    let proof = self.prover.prove_storage_branch(data)?;
                    let key = ProofKey::StorageEntry(query_id, data.block_number, data.position)
                        .to_string();
                    Some((key, proof))
                }
            },
            WorkerTaskType::StateEntry(data) => {
                let proof = self.prover.prove_state_db(task.contract, data)?;
                let key = ProofKey::StateDatabase(query_id, data.block_number).to_string();
                Some((key, proof))
            }
            WorkerTaskType::BlocksDb(input) => match input {
                BlocksDbData::BlockPartialNode(data) => {
                    let proof = self.prover.prove_block_partial_node(data)?;
                    let key = ProofKey::Aggregation(query_id, data.position).to_string();
                    Some((key, proof))
                }
                BlocksDbData::BlockFullNode(data) => {
                    let proof = self.prover.prove_block_full_node(data)?;
                    let key = ProofKey::Aggregation(query_id, data.position).to_string();
                    Some((key, proof))
                }
            },
            WorkerTaskType::Revelation(data) => {
                let proof = self.prover.prove_revelation(data)?;
                let key = ProofKey::Revelation(query_id).to_string();
                Some((key, proof))
            }
        };

        Ok(WorkerReply::new(0, proof))
    }
}
