#![feature(generic_const_exprs)]

use std::fmt::Display;
use std::fmt::Formatter;

use ethers::types::H256;
use serde::Deserialize;
use serde::Serialize;

pub mod v1;

pub type BlockNr = u64;
pub type TableId = u64;
pub type TableHash = u64;
pub type ChainId = u64;
pub type Proof = Vec<u8>;
pub type QueryId = String;
pub type RowKeyId = String;
pub type Identifier = u64;
pub type MptNodeVersion = (BlockNr, H256);

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "version")]
#[serde(rename_all = "snake_case")]
pub enum Message {
    /// Version 1 of the envelope format
    #[serde(rename = "1")]
    V1(v1::Envelope),

    /// Used by serde if the payload's version tag is not known.
    #[serde(other)]
    Unsupported,
}

impl Message {
    /// Creates a message using the `v1` format.
    pub fn v1(
        task_id: String,
        task: v1::Task,
        version: String,
    ) -> Self {
        Self::V1(v1::Envelope {
            task,
            task_id,
            mp2_version: version,
        })
    }

    /// Returns the task identifier.
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Message::V1(v1::Envelope { task_id, .. }) => Some(task_id),
            Message::Unsupported => None,
        }
    }

    /// Returns this message's task.
    pub fn task(&self) -> Option<&v1::Task> {
        match self {
            Message::V1(v1::Envelope { task, .. }) => Some(task),
            Message::Unsupported => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "version")]
#[serde(rename_all = "snake_case")]
pub enum Response {
    #[serde(rename = "1")]
    V1(v1::ReplyEnvelope),
}

impl Response {
    pub fn v1(
        task_id: String,
        proof: Proof,
    ) -> Self {
        Response::V1(v1::ReplyEnvelope::Proof { task_id, proof })
    }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ProverType {
    Unsupported,
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
                ProverType::Unsupported => "Unsupported",
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
