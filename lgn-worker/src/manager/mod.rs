use crate::metrics::Metrics;
use anyhow::bail;
use lgn_messages::types::{MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType};
use lgn_provers::provers::{LgnProver, ProverType};
use std::collections::HashMap;
use tracing::debug;

/// Manages provers for different proving task types
pub(crate) struct ProversManager<'a> {
    provers: HashMap<ProverType, Box<dyn LgnProver>>,
    metrics: &'a Metrics,
}

impl<'a> ProversManager<'a> {
    pub(crate) fn new(metrics: &'a Metrics) -> Self {
        Self {
            provers: HashMap::default(),
            metrics,
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
        let task_type: ProverType = envelope.inner().try_into()?;

        self.metrics
            .increment_tasks_received(task_type.to_string().as_str());

        match self.provers.get_mut(&task_type) {
            Some(prover) => {
                debug!("Running prover for task type: {task_type:?}");

                let start_time = std::time::Instant::now();

                let result = prover.run(envelope)?;

                self.metrics
                    .increment_tasks_processed(task_type.to_string().as_str());
                self.metrics.observe_task_processing_duration(
                    task_type.to_string().as_str(),
                    start_time.elapsed().as_secs_f64(),
                );

                return Ok(result);
            }
            None => {
                self.metrics
                    .increment_tasks_failed(task_type.to_string().as_str());
                bail!("No prover found for task type: {:?}", task_type);
            }
        }
    }
}
