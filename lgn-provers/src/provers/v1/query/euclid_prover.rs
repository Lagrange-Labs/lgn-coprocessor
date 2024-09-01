use crate::params::ParamsLoader;
use crate::provers::v1::query::prover::StorageQueryProver;
use alloy::primitives::U256;
use lgn_messages::types::v1::query::tasks::{
    FullNodeInput, PartialNodeInput, RowsEmbeddedProofInput, SinglePathBranchInput,
    SinglePathLeafInput,
};
use mp2_common::proof::ProofWithVK;
use mp2_v1::api::PublicParameters;
use parsil::assembler::DynamicCircuitPis;
use tracing::{debug, info};
use verifiable_db::api::{QueryCircuitInput, QueryParameters};
use verifiable_db::query::aggregation::SubProof;
use verifiable_db::query::api;
use verifiable_db::query::api::{CircuitInput, Parameters};
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation;

use super::MAX_NUM_COLUMNS;
use super::MAX_NUM_ITEMS_PER_OUTPUT;
use super::MAX_NUM_OUTPUTS;
use super::MAX_NUM_PLACEHOLDERS;
use super::MAX_NUM_PREDICATE_OPS;
use super::MAX_NUM_RESULTS;
use super::MAX_NUM_RESULT_OPS;

pub(crate) struct EuclidQueryProver {
    params: QueryParameters<
        MAX_NUM_COLUMNS,
        MAX_NUM_PREDICATE_OPS,
        MAX_NUM_RESULT_OPS,
        MAX_NUM_OUTPUTS,
        MAX_NUM_ITEMS_PER_OUTPUT,
        MAX_NUM_PLACEHOLDERS,
    >,
}

impl EuclidQueryProver {
    #[allow(dead_code)]
    pub fn new(
        params: QueryParameters<
            MAX_NUM_COLUMNS,
            MAX_NUM_PREDICATE_OPS,
            MAX_NUM_RESULT_OPS,
            MAX_NUM_OUTPUTS,
            MAX_NUM_ITEMS_PER_OUTPUT,
            MAX_NUM_PLACEHOLDERS,
        >,
    ) -> Self {
        Self { params }
    }

    #[allow(dead_code)]
    pub(crate) fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        skip_store: bool,
    ) -> anyhow::Result<Self> {
        debug!("Creating preprocessing prover");
        let params = ParamsLoader::prepare_bincode(
            url,
            dir,
            file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )?;
        debug!("Preprocessing prover created");
        Ok(Self { params })
    }
}

impl StorageQueryProver for EuclidQueryProver {
    fn prove_universal_circuit(
        &self,
        input: RowsEmbeddedProofInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving universal circuit");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_universal_circuit(
            &input.column_cells,
            &pis.predication_operations,
            &pis.result,
            &input.placeholders,
            input.is_leaf,
            &pis.bounds,
        )?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self.params.generate_proof(input)?;

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "universal circuit",
            "proof generation time: {:?}",
            now.elapsed()
        );
        info!("universal circuit size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_full_node(
        &self,
        embedded_tree_proof: Vec<u8>,
        left_child_proof: Vec<u8>,
        right_child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
        is_rows_tree_node: bool,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving full node");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_full_node(
            left_child_proof,
            right_child_proof,
            embedded_tree_proof,
            is_rows_tree_node,
            &pis.bounds,
        )?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self.params.generate_proof(input)?;

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "full node",
            "proof generation time: {:?}",
            now.elapsed()
        );

        info!("full node size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_partial_node(
        &self,
        input: PartialNodeInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving partial node");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_partial_node(
            input.proven_child_proof,
            embedded_proof,
            input.unproven_child_info,
            input.proven_child_position,
            input.is_rows_tree_node,
            &pis.bounds,
        )?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self.params.generate_proof(input)?;

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "partial node",
            "proof generation time: {:?}",
            now.elapsed()
        );

        info!("partial node size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_single_path_leaf(
        &self,
        input: SinglePathLeafInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving single path leaf");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_single_path(
            SubProof::new_embedded_tree_proof(embedded_proof)?,
            input.left_child_info,
            input.right_child_info,
            input.node_info,
            input.is_rows_tree_node,
            &pis.bounds,
        )?;

        let input = QueryCircuitInput::Query(circuit_input);

        let proof = self.params.generate_proof(input)?;

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "single path leaf",
            "proof generation time: {:?}",
            now.elapsed()
        );

        info!("single path leaf size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_single_path_branch(
        &self,
        input: SinglePathBranchInput,
        child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving single path branch");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_single_path(
            SubProof::new_child_proof(child_proof, input.child_position)?,
            input.left_child_info,
            input.right_child_info,
            input.node_info,
            input.is_rows_tree_node,
            &pis.bounds,
        )?;
        let input = QueryCircuitInput::Query(circuit_input);

        let proof = self.params.generate_proof(input)?;

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "single path branch",
            "proof generation time: {:?}",
            now.elapsed()
        );

        info!("single path branch size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        query_proof: Vec<u8>,
        indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Proving revelation");

        let now = std::time::Instant::now();

        let pis_hash = CircuitInput::<
            MAX_NUM_COLUMNS,
            MAX_NUM_PREDICATE_OPS,
            MAX_NUM_RESULT_OPS,
            MAX_NUM_RESULTS,
        >::ids_for_placeholder_hash(
            &pis.predication_operations,
            &pis.result,
            &placeholders,
            &pis.bounds,
        )?;
        let circuit_input = revelation::api::CircuitInput::new_revelation_no_results_tree(
            query_proof,
            indexing_proof,
            &pis.bounds,
            &placeholders,
            pis_hash,
        )?;

        let input = QueryCircuitInput::Revelation(circuit_input);

        let proof = self.params.generate_proof(input).unwrap();

        info!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "revelation",
            "proof generation time: {:?}",
            now.elapsed()
        );

        info!("revelation size in kB: {}", proof.len() / 1024);

        Ok(proof)
    }
}
