use mp2_v1::api::MAX_FIELD_PER_EVM;
use mp2_v1::values_extraction;
use serde_derive::Deserialize;
use serde_derive::Serialize;

use super::query::MAX_NUM_COLUMNS;
use super::ConcreteCircuitInput;

pub mod batched;

pub type ConcreteValueExtractionCircuitInput =
    values_extraction::CircuitInput<69, MAX_NUM_COLUMNS, MAX_FIELD_PER_EVM>;

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum PreprocessingTask {
    BatchedIndex(batched::BatchedIndex),
    BatchedLength(batched::BatchedLength),
    BatchedContract(batched::BatchedContract),
    CircuitInput(ConcreteCircuitInput),
}
