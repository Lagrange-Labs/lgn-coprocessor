use db_tasks::BatchedIndex;
use ext_tasks::BatchedContract;
use ext_tasks::BatchedLength;
use mp2_v1::api::MAX_FIELD_PER_EVM;
use mp2_v1::values_extraction;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::query::MAX_NUM_COLUMNS;
use super::ConcreteCircuitInput;

pub mod db_tasks;
pub mod ext_tasks;

pub type ConcreteValueExtractionCircuitInput =
    values_extraction::CircuitInput<69, MAX_NUM_COLUMNS, MAX_FIELD_PER_EVM>;

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum WorkerTaskType {
    BatchedIndex(BatchedIndex),
    BatchedLength(BatchedLength),
    BatchedContract(BatchedContract),
    CircuitInput(ConcreteCircuitInput),
}
