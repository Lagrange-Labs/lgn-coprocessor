pub(crate) mod v1;

use std::collections::HashMap;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

use anyhow::bail;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProverType;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::ToProverType;
use lgn_provers::provers::LgnProver;
use metrics::counter;
use metrics::histogram;
use tracing::info;

/// Manages provers for different proving task types
pub(crate) struct ProversManager {
    provers: HashMap<ProverType, Box<dyn LgnProver>>,
}

impl UnwindSafe for ProversManager {
}
impl RefUnwindSafe for ProversManager {
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
    pub(crate) fn add_prover(
        &mut self,
        task_type: ProverType,
        prover: Box<dyn LgnProver>,
    ) {
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
        &self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let prover_type: ProverType = envelope.inner.to_prover_type();

        counter!(
            "zkmr_worker_tasks_received_total",
            "task_type" => prover_type.to_string(),
        )
        .increment(1);

        match self.provers.get(&prover_type) {
            Some(prover) => {
                info!("Running prover for task type: {prover_type:?}");

                let start_time = std::time::Instant::now();

                let result = prover.run(envelope)?;

                counter!(
                    "zkmr_worker_tasks_processed_total",
                    "task_type" => prover_type.to_string(),
                )
                .increment(1);
                histogram!(
                    "zkmr_worker_task_processing_duration_seconds",
                    "task_type" => prover_type.to_string()
                )
                .record(start_time.elapsed().as_secs_f64());

                Ok(result)
            },
            None => {
                counter!(
                    "zkmr_worker_tasks_failed_total",
                    "task_type" => prover_type.to_string(),
                )
                .increment(1);
                bail!("No prover found for task type: {:?}", prover_type);
            },
        }
    }
}
