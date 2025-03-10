use anyhow::bail;
use lgn_messages::types::v1;
use lgn_messages::Proof;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::V1Prover;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct PreprocessingDummyProver;

impl V1Prover for PreprocessingDummyProver {
    fn run(
        &self,
        envelope: v1::Envelope,
    ) -> anyhow::Result<Proof> {
        match envelope.task {
            v1::Task::Preprocessing(..) => Ok(dummy_proof(PROOF_SIZE)),
            v1::Task::Query(..) => {
                bail!(
                    "PreprocessingDummyProver: unsupported task type. task_type: V1Query task_id: {}",
                    envelope.task_id,
                )
            },
            v1::Task::Groth16(_revelation_proof) => {
                bail!("PreprocessingDummyProver: unsupported task type. task_type: V1Groth16 task_id: {}", envelope.task_id)
            },
        }
    }
}
