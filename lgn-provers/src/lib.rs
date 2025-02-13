#![feature(generic_const_exprs)]
pub mod params;
pub mod provers;

#[cfg(feature = "dummy-prover")]
mod dummy_utils {
    /// Generates random data to be used as a dummy proof.
    pub fn dummy_proof(proof_size: usize) -> Vec<u8> {
        let data: Vec<_> = (0..proof_size).map(|_| rand::random::<u8>()).collect();
        bincode::serialize(&data).unwrap()
    }
}
