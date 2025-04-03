use std::fmt::Display;
use std::fmt::Formatter;

use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use thiserror::Error;

use crate::routing::RoutingKey;
use crate::KeyedPayload;

pub mod v1;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum TaskType {
    V1Preprocessing(v1::preprocessing::WorkerTask),
    V1Query(v1::query::WorkerTask),
    V1Groth16(v1::groth16::WorkerTask),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum ReplyType {
    V1Preprocessing(WorkerReply),
    V1Query(WorkerReply),
    V1Groth16(WorkerReply),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct MessageEnvelope {
    /// Query id is unique for each query and shared between all its tasks
    pub query_id: String,

    /// Task id is unique for each task and helps to map replies to tasks
    pub task_id: String,

    /// Task id referenced in the DB tasks table
    pub db_task_id: Option<i32>,

    /// Estimate how long it takes this task to finish.
    /// This includes may factors like: redis queue current length, workers count, parallel queries
    /// count, etc. Ideally assigned by an "intelligent" algorithm. Not important for now
    /// though. Might become relevant then we have clients waiting for results, and we can
    /// process queries relatively fast.
    pub rtt: u64,

    /// How much work prover has to do
    pub gas: Option<u64>,

    /// How and where to route the message.
    pub routing_key: RoutingKey,

    /// Details of the task to be executed.
    pub inner: TaskType,

    /// The proving system version
    pub version: String,
}

impl std::fmt::Debug for MessageEnvelope {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(
            f,
            "MSG#{:?}<{}, {}>",
            self.db_task_id.unwrap_or_default(),
            self.task_id,
            self.query_id
        )
    }
}

impl MessageEnvelope {
    pub fn new(
        query_id: String,
        task_id: String,
        inner: TaskType,
        routing_key: RoutingKey,
        version: String,
    ) -> Self {
        Self {
            query_id,
            inner,
            rtt: u64::MAX,
            gas: None,
            routing_key,
            task_id,
            db_task_id: None,
            version,
        }
    }

    pub fn query_id(&self) -> &str {
        &self.query_id
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.query_id, self.task_id)
    }

    pub fn inner(&self) -> &TaskType {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut TaskType {
        &mut self.inner
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct MessageReplyEnvelope {
    /// Query id is unique for each query and shared between all its tasks
    pub query_id: String,

    /// Task id is unique for each task and helps to map replies to tasks
    pub task_id: String,

    inner: ReplyType,

    error: Option<WorkerError>,
}

impl std::fmt::Debug for MessageReplyEnvelope {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "REPLY<{}, {}>", self.task_id, self.query_id)
    }
}

impl MessageReplyEnvelope {
    pub fn new(
        query_id: String,
        task_id: String,
        inner: ReplyType,
    ) -> Self {
        Self {
            query_id,
            task_id,
            inner,
            error: None,
        }
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.query_id, self.task_id)
    }

    /// Flatten `inner`, returning either Ok(successful_proof) or
    /// Err(WorkerError)
    pub fn inner(&self) -> Result<&ReplyType, &WorkerError> {
        match self.error.as_ref() {
            None => Ok(&self.inner),
            Some(t) => Err(t),
        }
    }

    /// Return the proof in this envelope, be it successful or not.
    pub fn content(&self) -> &ReplyType {
        &self.inner
    }

    pub fn query_id(&self) -> &str {
        &self.query_id
    }

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
    pub chain_id: u64,

    #[dbg(formatter = crate::types::kp_pretty)]
    pub proof: Option<KeyedPayload>,

    pub proof_type: ProofCategory,
}

impl WorkerReply {
    #[must_use]
    pub fn new(
        chain_id: u64,
        proof: Option<KeyedPayload>,
        proof_type: ProofCategory,
    ) -> Self {
        Self {
            chain_id,
            proof,
            proof_type,
        }
    }
}

#[derive(Error, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum WorkerError {
    // Start with general error to introduce the errors to replies
    #[error("{0}")]
    GeneralError(String),
}

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

pub fn kp_pretty(kp: &Option<KeyedPayload>) -> String {
    kp.as_ref()
        .map(|kp| kp.0.to_owned())
        .unwrap_or("empty".to_string())
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

impl TaskType {
    /// Returns [ProverType] which supports proving this [TaskType].
    ///
    /// This is used to dispatch the message to the correct underlying prover.
    pub fn to_prover_type(&self) -> ProverType {
        match self {
            TaskType::V1Preprocessing(_) => ProverType::V1Preprocessing,
            TaskType::V1Query(_) => ProverType::V1Query,
            TaskType::V1Groth16(_) => ProverType::V1Groth16,
        }
    }
}
