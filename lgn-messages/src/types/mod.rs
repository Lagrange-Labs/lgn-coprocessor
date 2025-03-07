use std::fmt::Display;
use std::fmt::Formatter;

use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use thiserror::Error;

pub mod v1;

const REQUIRED_STAKE_SMALL_USD: Stake = 98777;
const REQUIRED_STAKE_MEDIUM_USD: Stake = 98777;
const REQUIRED_STAKE_LARGE_USD: Stake = 169111;

pub type HashOutput = [u8; 32];

#[derive(Deserialize, Serialize)]
pub enum TaskType {
    V1Preprocessing(v1::preprocessing::WorkerTask),
    V1Query(v1::query::WorkerTaskType),

    /// Carries the plonky2 proof that will be wrapped on a groth16.
    V1Groth16(Vec<u8>),
}

#[derive(Deserialize, Serialize)]
pub struct MessageEnvelope {
    /// Identifier to relate proofs with tasks.
    pub task_id: String,

    /// The task to be proved.
    pub task: TaskType,

    /// The proving system version target version.
    ///
    /// Used to check the worker is compatible with the task.
    pub version: String,
}

impl std::fmt::Debug for MessageEnvelope {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("MessageEnvelope")
            .field("task_id", &self.task_id)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

impl MessageEnvelope {
    pub fn new(
        task_id: String,
        task: TaskType,
        version: String,
    ) -> Self {
        Self {
            task,
            task_id,
            version,
        }
    }

    /// Returns the task identifier.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    /// Returns this message's task.
    pub fn task(&self) -> &TaskType {
        &self.task
    }
}

#[derive(Deserialize, Serialize)]
pub struct MessageReplyEnvelope {
    /// The original task id.
    pub task_id: String,

    /// The proof result.
    reply: WorkerReply,

    /// Error details, if any.
    error: Option<WorkerError>,
}

impl std::fmt::Debug for MessageReplyEnvelope {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct("MessageReplyEnvelope")
            .field("task_id", &self.task_id)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl MessageReplyEnvelope {
    pub fn new(
        task_id: String,
        reply: WorkerReply,
    ) -> Self {
        Self {
            task_id,
            reply,
            error: None,
        }
    }

    /// Return the proof or the error if one occured.
    pub fn inner(&self) -> Result<&WorkerReply, &WorkerError> {
        match self.error.as_ref() {
            None => Ok(&self.reply),
            Some(t) => Err(t),
        }
    }

    /// Return the reply.
    pub fn reply(&self) -> &WorkerReply {
        &self.reply
    }

    /// Returns the task identifier.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

#[derive(Copy, Clone, Dbg, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProofCategory {
    Indexing,
    Querying,
}

#[derive(Clone, Dbg, PartialEq, Eq, Deserialize, Serialize)]
pub struct WorkerReply {
    pub proof: Option<Vec<u8>>,
    pub proof_type: ProofCategory,
}

impl WorkerReply {
    #[must_use]
    pub fn new(
        proof: Option<Vec<u8>>,
        proof_type: ProofCategory,
    ) -> Self {
        Self { proof, proof_type }
    }
}

#[derive(Error, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum WorkerError {
    // Start with general error to introduce the errors to replies
    #[error("{0}")]
    GeneralError(String),
}

pub type Stake = u128;

/// The segregation of job types according to their computational complexity
#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskDifficulty {
    // Due to the implicit ordering on which PartialOrd is built, this **MUST**
    // remain the smaller value at the top of the enum.
    // Hence, all workers of this class will always test .LT. *all* the tasks in
    // queue.
    /// Accept no tasks
    Disabled,
    /// Accept S tasks
    Small,
    /// Accept M tasks
    Medium,
    /// Accept L tasks
    Large,
}

impl TaskDifficulty {
    /// Returns the stake required in order to run such a task
    pub fn required_stake(&self) -> Stake {
        match self {
            TaskDifficulty::Small => REQUIRED_STAKE_SMALL_USD,
            TaskDifficulty::Medium => REQUIRED_STAKE_MEDIUM_USD,
            TaskDifficulty::Large => REQUIRED_STAKE_LARGE_USD,

            _ => 0,
        }
    }
}

impl Display for TaskDifficulty {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TaskDifficulty::Small => "small",
                TaskDifficulty::Medium => "medium",
                TaskDifficulty::Large => "large",
                TaskDifficulty::Disabled => "disabled",
            }
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProverType {
    V1Preprocessing,
    V1Query,
    V1Groth16,
}

impl Display for ProverType {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProverType::V1Preprocessing => "V1Preprocessing",
                ProverType::V1Query => "V1Query",
                ProverType::V1Groth16 => "V1Groth16",
            }
        )
    }
}

pub trait ToProverType {
    fn to_prover_type(&self) -> ProverType;
}

impl ToProverType for TaskType {
    fn to_prover_type(&self) -> ProverType {
        match self {
            TaskType::V1Preprocessing(_) => ProverType::V1Preprocessing,
            TaskType::V1Query(_) => ProverType::V1Query,
            TaskType::V1Groth16(_) => ProverType::V1Groth16,
        }
    }
}
