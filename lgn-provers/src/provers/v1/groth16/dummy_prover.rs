use anyhow::bail;
use lgn_messages::v1;
use lgn_messages::Proof;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::V1Prover;

const PROOF_SIZE: usize = 32;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct Groth16DummyProver;

impl V1Prover for Groth16DummyProver {
    fn run(
        &self,
        envelope: v1::Envelope,
    ) -> anyhow::Result<Proof> {
        match envelope.task {
            v1::Task::Preprocessing(..) => {
                bail!("Groth16DummyProver: unsupported task type. task_type: V1Preprocessing task_id: {}", envelope.task_id)
            },
            v1::Task::Query(..) => {
                bail!(
                    "Groth16DummyProver: unsupported task type. task_type: V1Query task_id: {}",
                    envelope.task_id,
                )
            },
            v1::Task::Groth16(_revelation_proof) => Ok(dummy_proof(PROOF_SIZE)),
        }
    }
}
