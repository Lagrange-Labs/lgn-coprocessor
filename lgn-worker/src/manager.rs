use std::collections::HashMap;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProverType;
use lgn_messages::types::TaskDifficulty;
use lgn_messages::types::ToProverType;
use lgn_provers::provers::LgnProver;
use metrics::counter;
use metrics::histogram;
use tracing::info;

use crate::config::Config;

/// Manages provers for different proving task types
pub(crate) struct ProversManager {
    provers: HashMap<ProverType, Box<dyn LgnProver>>,
}

impl UnwindSafe for ProversManager {
}
impl RefUnwindSafe for ProversManager {
}

impl ProversManager {
    pub(crate) fn new(
        config: &Config,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> anyhow::Result<Self> {
        info!("Registering the provers");

        let mut provers = HashMap::<ProverType, Box<dyn LgnProver>>::new();

        if config.worker.instance_type >= TaskDifficulty::Small {
            let query_prover = lgn_provers::provers::v1::query::create_prover(
                &config.public_params.params_base_url(),
                &config.public_params.dir,
                &config.public_params.query_params.file,
                checksums,
            )
            .context("initializing Small prover")?;

            provers.insert(ProverType::V1Query, Box::new(query_prover));
        }

        if config.worker.instance_type >= TaskDifficulty::Medium {
            let preprocessing_prover = lgn_provers::provers::v1::preprocessing::create_prover(
                &config.public_params.params_base_url(),
                &config.public_params.dir,
                &config.public_params.preprocessing_params.file,
                checksums,
            )
            .context("initializing Medium prover")?;

            provers.insert(ProverType::V1Preprocessing, Box::new(preprocessing_prover));
        }

        if config.worker.instance_type >= TaskDifficulty::Large {
            let groth16_prover = lgn_provers::provers::v1::groth16::create_prover(
                &config.public_params.params_base_url(),
                &config.public_params.dir,
                &config.public_params.groth16_assets.circuit_file,
                checksums,
                &config.public_params.groth16_assets.r1cs_file,
                &config.public_params.groth16_assets.pk_file,
            )
            .context("initializing Large prover")?;

            provers.insert(ProverType::V1Groth16, Box::new(groth16_prover));
        }

        info!("Finished registering the provers.");

        Ok(Self { provers })
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
        envelope: MessageEnvelope,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let prover_type: ProverType = envelope.task.to_prover_type();

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
