use crate::provers::v1::query::prover::StorageQueryProver;
use lgn_messages::types::v1::query::tasks::{
    NonExistenceInput, PartialNodeInput, RowsEmbeddedProofInput, SinglePathBranchInput,
    SinglePathLeafInput,
};
use parsil::assembler::DynamicCircuitPis;
use std::thread::sleep;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;

pub(crate) struct DummyProver;

impl StorageQueryProver for DummyProver {
    fn prove_universal_circuit(
        &self,
        _input: RowsEmbeddedProofInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_full_node(
        &self,
        _embedded_tree_proof: Vec<u8>,
        _left_child_proof: Vec<u8>,
        _right_child_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
        _is_rows_tree_node: bool,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_partial_node(
        &self,
        _input: PartialNodeInput,
        _embedded_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_single_path_leaf(
        &self,
        _input: SinglePathLeafInput,
        _embedded_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_single_path_branch(
        &self,
        _input: SinglePathBranchInput,
        _child_proof: Vec<u8>,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_revelation(
        &self,
        _pis: &DynamicCircuitPis,
        _placeholders: Placeholders,
        _query_proof: Vec<u8>,
        _indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }

    fn prove_non_existence(
        &self,
        _input: NonExistenceInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(prove())
    }
}

#[allow(dead_code)]
fn prove() -> Vec<u8> {
    sleep(std::time::Duration::from_millis(100));
    let data: Vec<_> = (0..120).map(|_| rand::random::<u8>()).collect();
    bincode::serialize(&data).unwrap()
}
