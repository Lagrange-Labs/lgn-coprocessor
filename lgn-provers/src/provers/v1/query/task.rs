use crate::provers::v1::query::prover::StorageQueryProver;
use crate::provers::LgnProver;
use lgn_messages::types::v1::preprocessing::{db_keys, ext_keys, WorkerTaskType};
use lgn_messages::types::v1::query::tasks::WorkerTask;
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, WorkerReply,
};

pub struct Querying<P> {
    prover: P,
}

impl<P: StorageQueryProver> LgnProver<TaskType, ReplyType> for Querying<P> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Query(task @ WorkerTask { chain_id, .. }) = envelope.inner {
            let key = todo!();
            let result = self.run_inner(task)?;
            let reply_type =
                ReplyType::V1Preprocessing(WorkerReply::new(chain_id, Some((key, result))));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }
}

impl<P: StorageQueryProver> Querying<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }

    pub fn run_inner(&mut self, task: WorkerTask) -> anyhow::Result<Vec<u8>> {
        todo!("Implement Querying::run_inner")
    }
}
