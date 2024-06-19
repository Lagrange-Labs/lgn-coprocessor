use anyhow::anyhow;
use lgn_messages::types::{MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType};
use std::fmt::Display;

pub mod v0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProverType {
    /// V0 query preprocessing handler.
    Query2Preprocess,

    /// V0 query handler.
    Query2Query,

    QueryErc20,

    /// V0 Groth16 handler.
    Query2Groth16,
}

impl TryFrom<&TaskType> for ProverType {
    type Error = anyhow::Error;

    fn try_from(task_type: &TaskType) -> anyhow::Result<Self> {
        match task_type {
            TaskType::StoragePreprocess(_) => Ok(Self::Query2Preprocess),
            TaskType::StorageQuery(_) => Ok(Self::Query2Query),
            TaskType::StorageGroth16(_) => Ok(Self::Query2Groth16),
            TaskType::Erc20Query(_) => Ok(Self::QueryErc20),
            _ => Err(anyhow!("Unsupported task type: {:?}", task_type)),
        }
    }
}

impl Display for ProverType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query2Preprocess => write!(f, "Query2Preprocess"),
            Self::Query2Query => write!(f, "Query2Query"),
            Self::Query2Groth16 => write!(f, "Query2Groth16"),
            Self::QueryErc20 => write!(f, "QueryErc20"),
        }
    }
}

/// The prover trait that accepts [`MessageEnvelope`] and is able to process tasks of type [`TaskType`].
pub trait LgnProver {
    /// Run the prover with the given [`MessageEnvelope`] and return the result as a [`MessageReplyEnvelope`].
    ///
    /// # Arguments
    /// * `envelope` - The [`MessageEnvelope`] that contains the task to be processed.
    ///
    /// # Returns
    /// The result of processing the task as a [`MessageReplyEnvelope`].
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>>;
}
