use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;

pub mod v1;

/// The prover trait that accepts [`MessageEnvelope`] and is able to process tasks of type
/// [`TaskType`].
pub trait LgnProver {
    /// Run the prover with the given [`MessageEnvelope`] and return the result as a
    /// [`MessageReplyEnvelope`].
    ///
    /// # Arguments
    /// * `envelope` - The [`MessageEnvelope`] that contains the task to be processed.
    ///
    /// # Returns
    /// The result of processing the task as a [`MessageReplyEnvelope`].
    fn run(
        &self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>>;
}
