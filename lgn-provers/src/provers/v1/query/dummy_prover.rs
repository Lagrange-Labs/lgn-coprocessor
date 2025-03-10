use anyhow::bail;
use lgn_messages::types::v1;
use lgn_messages::types::v1::query::WorkerTaskType;
use lgn_messages::Proof;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::V1Prover;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct QueryDummyProver;

impl V1Prover for QueryDummyProver {
    fn run(
        &self,
        envelope: v1::Envelope,
    ) -> anyhow::Result<Proof> {
        match envelope.task {
            v1::Task::Preprocessing(..) => {
                bail!("QueryDummyProver: unsupported task type. task_type: V1Preprocessing task_id: {}", envelope.task_id)
            },
            v1::Task::Query(WorkerTaskType::Query(..)) => {
                let proof = dummy_proof(PROOF_SIZE);
                Ok(proof)
            },
            v1::Task::Groth16(..) => {
                bail!(
                    "QueryDummyProver: unsupported task type. task_type: V1Groth16 task_id: {}",
                    envelope.task_id,
                )
            },
        }
    }
}
