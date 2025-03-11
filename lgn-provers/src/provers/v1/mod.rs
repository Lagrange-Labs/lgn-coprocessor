use lgn_messages::v1;
use lgn_messages::Proof;

pub mod groth16;
pub mod preprocessing;
pub mod query;

/// Prover for V1 of the protocol
pub trait V1Prover {
    fn run(
        &self,
        envelope: v1::Envelope,
    ) -> anyhow::Result<Proof>;
}
