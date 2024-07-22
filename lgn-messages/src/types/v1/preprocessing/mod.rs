use crate::types::v1::preprocessing::task::{WorkerTask, WorkerTaskType};
use crate::types::{ProverType, ToProverType};

pub mod keys;
pub mod reply;
pub mod task;

impl ToProverType for WorkerTask {
    fn to_prover_type(&self) -> ProverType {
        match self.task_type {
            WorkerTaskType::Extraction(_) => ProverType::PreprocessingV1,
            WorkerTaskType::Database(_) => panic!("Unsupported task type: {:?}", self),
        }
    }
}
