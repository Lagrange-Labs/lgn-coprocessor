use std::collections::HashMap;
use std::time::Instant;

use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use lgn_messages::types::v1;
use lgn_messages::Proof;
use tracing::info;

use crate::params;
use crate::provers::v1::V1Prover;

#[derive(Debug)]
pub struct Groth16Prover {
    inner: groth16_framework_v1::Groth16Prover,
}

impl Groth16Prover {
    /// Initialize the Groth16 prover from bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn init(
        url: &str,
        dir: &str,
        circuit_file: &str,
        r1cs_file: &str,
        pk_file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> Result<Self> {
        let circuit_bytes = params::prepare_raw(url, dir, circuit_file, checksums)?;
        let r1cs_bytes = params::prepare_raw(url, dir, r1cs_file, checksums)?;
        let pk_bytes = params::prepare_raw(url, dir, pk_file, checksums)?;

        info!("Creating Groth16 prover");
        let inner = groth16_framework_v1::Groth16Prover::from_bytes(
            r1cs_bytes.to_vec(),
            pk_bytes.to_vec(),
            circuit_bytes.to_vec(),
        )?;
        info!("Groth16 prover created");

        Ok(Self { inner })
    }
}

impl V1Prover for Groth16Prover {
    fn run(
        &self,
        envelope: v1::Envelope,
    ) -> anyhow::Result<Proof> {
        match envelope.task {
            v1::Task::Preprocessing(..) => {
                bail!(
                    "Groth16: unsupported task type. task_type: V1Preprocessing task_id: {}",
                    envelope.task_id,
                )
            },
            v1::Task::Query(..) => {
                bail!(
                    "Groth16: unsupported task type. task_type: V1Query task_id: {}",
                    envelope.task_id,
                )
            },
            v1::Task::Groth16(revelation_proof) => {
                let now = Instant::now();
                let proof = self
                    .inner
                    .prove(revelation_proof.as_slice())
                    .with_context(|| {
                        format!(
                            "Failed to generate the Groth16 proof. task_id: {}",
                            envelope.task_id,
                        )
                    })?;

                info!(
                    time = now.elapsed().as_secs_f32(),
                    proof_type = "groth16",
                    "Finish generating the Groth16 proof. task_id: {} elapsed: {:?}",
                    envelope.task_id,
                    now.elapsed(),
                );

                Ok(proof)
            },
        }
    }
}
