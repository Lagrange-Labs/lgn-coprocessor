use mp2_v1::api::CircuitInput;
use mp2_v1::api::PublicParameters;
use query::MAX_NUM_COLUMNS;

pub mod preprocessing;
pub mod query;

pub type ConcretePublicParameters = PublicParameters<MAX_NUM_COLUMNS>;
pub type ConcreteCircuitInput = CircuitInput<MAX_NUM_COLUMNS>;
