use lgn_messages::types::v1::preprocessing::db_keys;
use lgn_messages::types::v1::preprocessing::ext_keys;
use lgn_messages::types::v1::preprocessing::WorkerTask;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;

use crate::dummy_utils::dummy_proof;
use crate::provers::LgnProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl LgnProver for DummyProver {
    fn run(
        &self,
        envelope: &MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Preprocessing(task @ WorkerTask { chain_id, .. }) = &envelope.inner {
            let key = match &task.task_type {
                WorkerTaskType::Extraction(_) => {
                    let key: ext_keys::ProofKey = task.into();
                    key.to_string()
                },
                WorkerTaskType::Database(_) => {
                    let key: db_keys::ProofKey = task.into();
                    key.to_string()
                },
            };
            let result = dummy_proof(PROOF_SIZE);
            let reply_type = ReplyType::V1Preprocessing(WorkerReply::new(
                *chain_id,
                Some((key, result)),
                ProofCategory::Querying,
            ));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }
}
