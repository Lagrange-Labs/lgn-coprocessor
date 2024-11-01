pub mod keys;

use std::collections::HashSet;

use serde_derive::Deserialize;
use serde_derive::Serialize;

type LogMaxCapacity = u8;
type LogSubsetSize = u8;
type LeavesIndices = HashSet<usize>;
type Level = usize;
type Index = usize;

pub const ROUTING_DOMAIN: &str = "recproof";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct WorkerTask
{
    /// What stage of proof generation process is this task.
    pub task_type: WorkerTaskType,

    /// Which experiment is this task for.
    pub experiment: Experiment,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub enum WorkerTaskType
{
    Prepare(
        LogMaxCapacity,
        LogSubsetSize,
        LeavesIndices,
    ),
    Compute(
        (
            Level,
            Index,
        ),
        LogMaxCapacity,
        LogSubsetSize,
    ),
    BatchCompute(
        Vec<(
            Level,
            Index,
        )>,
        LogMaxCapacity,
        LogSubsetSize,
    ),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Experiment
{
    Inclusion,
    DigestTranslation,
    Bucketing,
    BlsBucketing,
}

impl From<String> for Experiment
{
    fn from(experiment: String) -> Self
    {
        match experiment.as_str()
        {
            "inclusion" => Self::Inclusion,
            "digest_translation" => Self::DigestTranslation,
            "bucketing" => Self::Bucketing,
            "bls_bucketing" => Self::BlsBucketing,
            _ => panic!("Unknown experiment: {experiment}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WorkerReply
{
    pub query_id: String,
    pub task_id: String,
}

impl WorkerReply
{
    /// Initializes a new worker reply.
    ///
    /// # Arguments
    /// * `query_id` - Query id this reply is for.
    /// * `task_id` - Task id this reply is for.
    #[must_use]
    pub fn new(
        query_id: String,
        task_id: String,
    ) -> Self
    {
        Self {
            query_id,
            task_id,
        }
    }
}
