use lgn_messages::types::v1::query::tasks::{
    EmbeddedProofInput, FullNodeInput, PartialNodeInput, SinglePathBranchInput, SinglePathLeafInput,
};
use parsil::assembler::DynamicCircuitPis;

pub trait StorageQueryProver {
    fn prove_universal_circuit(
        &self,
        input: EmbeddedProofInput,
        pis: DynamicCircuitPis,
        is_leaf: bool,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_full_node(
        &self,
        embedded_tree_proof: Vec<u8>,
        left_child_proof: Vec<u8>,
        right_child_proof: Vec<u8>,
        input: FullNodeInput,
        pis: DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_partial_node(
        &self,
        input: PartialNodeInput,
        child_proof: Vec<u8>,
        embedded_proof: Vec<u8>,
        pis: DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_single_path_leaf(
        &self,
        input: SinglePathLeafInput,
        embedded_proof: Vec<u8>,
        pis: DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_single_path_branch(
        &self,
        input: SinglePathBranchInput,
        child_proof: Vec<u8>,
        pis: DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;
}
