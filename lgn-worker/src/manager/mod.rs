use anyhow::anyhow;

use lgn_messages::types::{MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType};
use lgn_provers::provers::{LgnProver, ProverType};
use std::collections::HashMap;
use tracing::debug;

/// Manages provers for different proving task types
pub(crate) struct ProversManager {
    provers: HashMap<ProverType, Box<dyn LgnProver>>,
}

impl ProversManager {
    pub(crate) fn new() -> Self {
        Self {
            provers: HashMap::default(),
        }
    }

    /// Registers a new prover.
    ///
    /// # Arguments
    /// * `task_type` - The type of task the prover can process
    /// * `prover` - The prover that can process the task type specified by `task_type`
    pub(crate) fn add_prover(&mut self, task_type: ProverType, prover: Box<dyn LgnProver>) {
        self.provers.insert(task_type, prover);
    }

    /// Sends proving request to a matching prover
    ///
    /// # Arguments
    /// * `envelope` - The message envelope containing the task to be processed
    ///
    /// # Returns
    /// A message reply envelope containing the result of the proving task
    pub(crate) fn delegate_proving(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let task_type = envelope.inner().try_into()?;
        let prover = self
            .provers
            .get_mut(&task_type)
            .ok_or_else(|| anyhow!("No handler for task type: {task_type:?}"))?;

        debug!("Running prover for task type: {task_type:?}");

        prover.run(envelope)
    }
}
