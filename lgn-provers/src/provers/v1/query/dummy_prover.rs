use anyhow::bail;
use lgn_messages::types::v1::query::keys::ProofKey;
use lgn_messages::types::v1::query::WorkerTask;
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
pub struct QueryDummyProver;

impl LgnProver for QueryDummyProver {
    fn run(
        &self,
        envelope: RequestVersioned,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id().to_owned();
        let task_id = envelope.task_id().to_owned();

        if let TaskType::V1Query(ref task @ WorkerTask { chain_id, .. }) = envelope.inner() {
            let key: ProofKey = task.into();
            let result = dummy_proof(PROOF_SIZE);
            let reply_type = ReplyType::V1Query(WorkerReply::new(
                *chain_id,
                Some((key.to_string(), result)),
                ProofCategory::Querying,
            ));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            bail!("Unexpected task: {:?}", envelope);
        }
    }
}
