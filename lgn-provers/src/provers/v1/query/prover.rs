use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::PartialNodeInput;
use lgn_messages::types::v1::query::tasks::RowsEmbeddedProofInput;
use lgn_messages::types::v1::query::tasks::SinglePathBranchInput;
use lgn_messages::types::v1::query::tasks::SinglePathLeafInput;
use parsil::assembler::DynamicCircuitPis;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;

pub trait StorageQueryProver
{
    fn prove_universal_circuit(
        &self,
        input: RowsEmbeddedProofInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_full_node(
        &self,
        embedded_tree_proof: Vec<u8>,
        left_child_proof: Vec<u8>,
        right_child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
        is_rows_tree_node: bool,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_partial_node(
        &self,
        input: PartialNodeInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_single_path_leaf(
        &self,
        input: SinglePathLeafInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_single_path_branch(
        &self,
        input: SinglePathBranchInput,
        child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        query_proof: Vec<u8>,
        indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_non_existence(
        &self,
        input: NonExistenceInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;
}
