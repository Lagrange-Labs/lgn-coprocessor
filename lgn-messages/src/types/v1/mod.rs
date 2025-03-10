use std::fmt::Formatter;

use mp2_v1::api::CircuitInput;
use mp2_v1::api::PublicParameters;
use query::MAX_NUM_COLUMNS;
use serde::Deserialize;
use serde::Serialize;

use super::ProverType;
use super::ToProverType;

pub mod preprocessing;
pub mod query;

pub type ConcretePublicParameters = PublicParameters<MAX_NUM_COLUMNS>;
pub type ConcreteCircuitInput = CircuitInput<MAX_NUM_COLUMNS>;

/// Envelop for v1 messages.
#[derive(Deserialize, Serialize)]
pub struct Envelope {
    /// Identifier to relate proofs with tasks.
    pub task_id: String,

    /// The task to be proved.
    pub task: Task,

    /// The proving system version target version.
    ///
    /// Used to check the worker is compatible with the task.
    pub mp2_version: String,
}

impl std::fmt::Debug for Envelope {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("MessageEnvelope")
            .field("task_id", &self.task_id)
            .field("mp2_version", &self.mp2_version)
            .finish_non_exhaustive()
    }
}

/// The different possible task for V1.
///
/// Each task type requires a different node class.
///
/// - `TaskType::Queries` requires a small node.
/// - `TaskType::Preprocessing` requires a medium node.
/// - `TaskType::Groth16` requires a large node.
#[derive(Deserialize, Serialize)]
pub enum Task {
    /// Preprocessing tasks.
    ///
    /// These tasks include tasks for extracting data and building
    /// the verifiable database.
    Preprocessing(preprocessing::WorkerTaskType),

    /// Query tasks.
    ///
    /// Tasks to query the verifiable database.
    Query(query::WorkerTaskType),

    /// Task to wrap a query result in a final groth16 proof.
    Groth16(Vec<u8>),
}

impl ToProverType for Task {
    fn to_prover_type(&self) -> ProverType {
        match self {
            Task::Preprocessing(_) => ProverType::V1Preprocessing,
            Task::Query(_) => ProverType::V1Query,
            Task::Groth16(_) => ProverType::V1Groth16,
        }
    }
}
