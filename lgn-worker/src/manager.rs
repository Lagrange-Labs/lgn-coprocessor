use std::collections::HashMap;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;

use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProverType;
use lgn_messages::types::TaskDifficulty;
use lgn_provers::provers::LgnProver;
use tokio::task::JoinSet;
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
    /// Initialise the proves.
    ///
    /// This will:
    /// - Download the public parameters if necessary
    /// - Create the corresponding workers, as determined by the worker's configuration
    pub(crate) async fn new(
        config: &Config,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> anyhow::Result<Self> {
        info!("Registering the provers");

        let mut join_set: JoinSet<anyhow::Result<(ProverType, Box<dyn LgnProver + Send>)>> =
            JoinSet::new();

        if config.worker.instance_type >= TaskDifficulty::Small {
            let config = config.clone();
            let checksums = checksums.clone();
            join_set.spawn(async move {
                let query_prover = lgn_provers::provers::v1::query::create_prover(
                    &config.public_params.params_base_url(),
                    &config.public_params.dir,
                    &config.public_params.query_params.file,
                    &checksums,
                )
                .await
                .context("initializing Small prover")?;

                let query_prover: Box<dyn LgnProver + Send> = Box::new(query_prover);
                Ok((ProverType::V1Query, query_prover))
            });
        }

        if config.worker.instance_type >= TaskDifficulty::Medium {
            let config = config.clone();
            let checksums = checksums.clone();
            join_set.spawn(async move {
                let preprocessing_prover = lgn_provers::provers::v1::preprocessing::create_prover(
                    &config.public_params.params_base_url(),
                    &config.public_params.dir,
                    &config.public_params.preprocessing_params.file,
                    &checksums,
                )
                .await
                .context("initializing Medium prover")?;

                let preprocessing_prover: Box<dyn LgnProver + Send> =
                    Box::new(preprocessing_prover);
                Ok((ProverType::V1Preprocessing, preprocessing_prover))
            });
        }

        if config.worker.instance_type >= TaskDifficulty::Large {
            let config = config.clone();
            let checksums = checksums.clone();
            join_set.spawn(async move {
                let groth16_prover = lgn_provers::provers::v1::groth16::create_prover(
                    &config.public_params.params_base_url(),
                    &config.public_params.dir,
                    &config.public_params.groth16_assets.circuit_file,
                    &checksums,
                    &config.public_params.groth16_assets.r1cs_file,
                    &config.public_params.groth16_assets.pk_file,
                )
                .await
                .context("initializing Large prover")?;

                let groth16_prover: Box<dyn LgnProver + Send> = Box::new(groth16_prover);
                Ok((ProverType::V1Groth16, groth16_prover))
            });
        }

        let mut provers = HashMap::<ProverType, Box<dyn LgnProver>>::new();

        while let Some(res) = join_set.join_next().await {
            let (prover_type, prover) = res??;
            provers.insert(prover_type, prover);
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
        let prover_type: ProverType = envelope.inner.to_prover_type();

        match self.provers.get(&prover_type) {
            Some(prover) => prover.run(envelope),
            None => {
                bail!(
                    "No prover found for task type. prover_type: {:?}",
                    prover_type
                );
            },
        }
    }
}
