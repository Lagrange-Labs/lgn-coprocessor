use anyhow::bail;
use lgn_messages::types::v1::preprocessing::db_keys;
use lgn_messages::types::v1::preprocessing::ext_keys;
use lgn_messages::types::v1::preprocessing::WorkerTask;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::RequestVersioned;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;

use crate::dummy_utils::dummy_proof;
use crate::provers::LgnProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct PreprocessingDummyProver;

impl LgnProver for PreprocessingDummyProver {
    fn run(
        &self,
        envelope: RequestVersioned,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id().to_owned();
        let task_id = envelope.task_id().to_owned();
        if let TaskType::V1Preprocessing(task @ WorkerTask { chain_id, .. }) = envelope.into_inner()
        {
            let key = match &task.task_type {
                WorkerTaskType::Extraction(_) => {
                    let key: ext_keys::ProofKey = (&task).into();
                    key.to_string()
                },
                WorkerTaskType::Database(_) => {
                    let key: db_keys::ProofKey = (&task).into();
                    key.to_string()
                },
            };
            let result = dummy_proof(PROOF_SIZE);
            let reply_type = ReplyType::V1Preprocessing(WorkerReply::new(
                chain_id,
                Some((key, result)),
                ProofCategory::Querying,
            ));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            bail!("Unexpected task. task_id: {}", task_id);
        }
    }
}
