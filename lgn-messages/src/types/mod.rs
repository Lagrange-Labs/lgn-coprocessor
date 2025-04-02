use std::fmt::Display;
use std::fmt::Formatter;

use derive_debug_plus::Dbg;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use thiserror::Error;

use crate::routing::RoutingKey;

pub mod v1;

const REQUIRED_STAKE_SMALL_USD: Stake = 98777;
const REQUIRED_STAKE_MEDIUM_USD: Stake = 98777;
const REQUIRED_STAKE_LARGE_USD: Stake = 169111;

/// A keyed payload contains a bunch of bytes accompanied by a storage index
pub type KeyedPayload = (String, Vec<u8>);

pub trait ToKeyedPayload {
    fn to_keyed_payload(&self) -> KeyedPayload;
}

pub type HashOutput = [u8; 32];

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

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct MessageEnvelope<T> {
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
    pub inner: T,

    /// The proving system version
    pub version: String,
}
impl<T> std::fmt::Debug for MessageEnvelope<T> {
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

impl<T> MessageEnvelope<T> {
    pub fn new(
        query_id: String,
        task_id: String,
        inner: T,
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

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct MessageReplyEnvelope<T> {
    /// Query id is unique for each query and shared between all its tasks
    pub query_id: String,

    /// Task id is unique for each task and helps to map replies to tasks
    pub task_id: String,

    inner: T,

    error: Option<WorkerError>,
}
impl<T> std::fmt::Debug for MessageReplyEnvelope<T> {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "REPLY<{}, {}>", self.task_id, self.query_id)
    }
}

impl<T> MessageReplyEnvelope<T> {
    pub fn new(
        query_id: String,
        task_id: String,
        inner: T,
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
    pub fn inner(&self) -> Result<&T, &WorkerError> {
        match self.error.as_ref() {
            None => Ok(&self.inner),
            Some(t) => Err(t),
        }
    }

    /// Return the proof in this envelope, be it successful or not.
    pub fn content(&self) -> &T {
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

#[derive(
    Default, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize,
)]
pub struct Position {
    pub level: usize,
    pub index: usize,
}

impl Position {
    #[must_use]
    pub fn new(
        level: usize,
        index: usize,
    ) -> Self {
        Self { level, index }
    }

    pub fn as_tuple(&self) -> (usize, usize) {
        (self.level, self.index)
    }
}

impl Display for Position {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "{}/{}", self.level, self.index)
    }
}

impl From<(usize, usize)> for Position {
    fn from((level, index): (usize, usize)) -> Self {
        Self { level, index }
    }
}

impl From<Position> for (usize, usize) {
    fn from(position: Position) -> Self {
        (position.level, position.index)
    }
}

/// All the messages that may transit from the worker to the server
#[derive(Debug, Serialize, Deserialize)]
pub enum UpstreamPayload<T> {
    /// The worker is authenticating
    Authentication { token: String },

    /// The worker is ready to start working(after params loading)
    Ready,

    /// the workers sends back a proof for the given task ID
    Done(MessageReplyEnvelope<T>),

    /// the worker encountered an error when computing the proof
    ProvingError(String),
}

impl<T> Display for UpstreamPayload<T> {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            UpstreamPayload::Done(_) => {
                write!(f, "Task done")
            },
            UpstreamPayload::Authentication { .. } => {
                write!(f, "Authentication")
            },
            UpstreamPayload::Ready => {
                write!(f, "Ready")
            },
            UpstreamPayload::ProvingError(_) => {
                write!(f, "Proving error")
            },
        }
    }
}

/// All the messages that may transit from the server to the worker
#[derive(Debug, Serialize, Deserialize)]
pub enum DownstreamPayload<T> {
    /// indicate a successful authentication to the worker
    Ack,
    /// order the worker to process the given task
    Todo { envelope: MessageEnvelope<T> },
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

    /// Returns the minimal worker class required to process a task of the given queue
    pub fn from_queue(domain: &str) -> Self {
        let domain = domain.split('_').next().expect("invalid routing key");
        match domain {
            v1::preprocessing::ROUTING_DOMAIN => TaskDifficulty::Medium,
            v1::query::ROUTING_DOMAIN => TaskDifficulty::Small,
            v1::groth16::ROUTING_DOMAIN => TaskDifficulty::Large,
            _ => panic!("unknown routing domain"),
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
                TaskDifficulty::Disabled => "disbaled",
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
    /// V0 query preprocessing handler.
    Query2Preprocess,

    /// V0 query handler.
    Query2Query,

    QueryErc20,

    /// V0 Groth16 handler.
    Query2Groth16,

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
                ProverType::Query2Preprocess => "Query2Preprocess",
                ProverType::Query2Query => "Query2Query",
                ProverType::Query2Groth16 => "Query2Groth16",
                ProverType::QueryErc20 => "QueryErc20",
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
