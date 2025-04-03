use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;

pub mod v1;

/// Prover trait.
///
/// Implementors of this trait shall support some of the message types, for each
/// support message types it must generate a proof.
pub trait LgnProver {
    fn run(
        &self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>>;
}
