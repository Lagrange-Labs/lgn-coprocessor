use crate::{dummy_utils::dummy_proof, provers::v1::query::prover::StorageQueryProver};
use lgn_messages::types::v1::query::tasks::{
    NonExistenceInput, PartialNodeInput, RowsEmbeddedProofInput, SinglePathBranchInput,
    SinglePathLeafInput,
};
use parsil::assembler::DynamicCircuitPis;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl StorageQueryProver for DummyProver {
    fn prove_universal_circuit(
        &self,
        _input: RowsEmbeddedProofInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_full_node(
        &self,
        _embedded_tree_proof: Vec<u8>,
        _left_child_proof: Vec<u8>,
        _right_child_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
        _is_rows_tree_node: bool,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_partial_node(
        &self,
        _input: PartialNodeInput,
        _embedded_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_single_path_leaf(
        &self,
        _input: SinglePathLeafInput,
        _embedded_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_single_path_branch(
        &self,
        _input: SinglePathBranchInput,
        _child_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_revelation(
        &self,
        _pis: &DynamicCircuitPis,
        _placeholders: Placeholders,
        _query_proof: Vec<u8>,
        _indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_non_existence(
        &self,
        _input: NonExistenceInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }
}
