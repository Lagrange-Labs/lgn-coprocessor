use anyhow::Context;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::PartialNodeInput;
use lgn_messages::types::v1::query::tasks::RowsEmbeddedProofInput;
use lgn_messages::types::v1::query::tasks::SinglePathBranchInput;
use lgn_messages::types::v1::query::tasks::SinglePathLeafInput;
use metrics::histogram;
use parsil::assembler::DynamicCircuitPis;
use tracing::debug;
use tracing::info;
use verifiable_db::api::QueryCircuitInput;
use verifiable_db::api::QueryParameters;
use verifiable_db::query::aggregation::QueryHashNonExistenceCircuits;
use verifiable_db::query::aggregation::SubProof;
use verifiable_db::query::api::CircuitInput;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation::api::MatchingRow;
use verifiable_db::revelation::{
    self,
};

use super::prover::StorageQueryProver;
use super::INDEX_TREE_MAX_DEPTH;
use super::MAX_NUM_COLUMNS;
use super::MAX_NUM_ITEMS_PER_OUTPUT;
use super::MAX_NUM_OUTPUTS;
use super::MAX_NUM_PLACEHOLDERS;
use super::MAX_NUM_PREDICATE_OPS;
use super::MAX_NUM_RESULT_OPS;
use super::ROW_TREE_MAX_DEPTH;
use crate::params::ParamsLoader;

pub(crate) struct EuclidQueryProver
{
    params: QueryParameters<
        ROW_TREE_MAX_DEPTH,
        INDEX_TREE_MAX_DEPTH,
        MAX_NUM_COLUMNS,
        MAX_NUM_PREDICATE_OPS,
        MAX_NUM_RESULT_OPS,
        MAX_NUM_OUTPUTS,
        MAX_NUM_ITEMS_PER_OUTPUT,
        MAX_NUM_PLACEHOLDERS,
    >,
}

impl EuclidQueryProver
{
    #[allow(dead_code)]
    pub fn new(
        params: QueryParameters<
            ROW_TREE_MAX_DEPTH,
            INDEX_TREE_MAX_DEPTH,
            MAX_NUM_COLUMNS,
            MAX_NUM_PREDICATE_OPS,
            MAX_NUM_RESULT_OPS,
            MAX_NUM_OUTPUTS,
            MAX_NUM_ITEMS_PER_OUTPUT,
            MAX_NUM_PLACEHOLDERS,
        >
    ) -> Self
    {
        Self {
            params,
        }
    }

    pub(crate) fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        skip_store: bool,
    ) -> anyhow::Result<Self>
    {
        debug!("Creating preprocessing prover");
        let params = ParamsLoader::prepare_bincode(
            url,
            dir,
            file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )
        .context("while loading bincode-serialized parameters")?;
        debug!("Preprocessing prover created");
        Ok(
            Self {
                params,
            },
        )
    }
}

impl StorageQueryProver for EuclidQueryProver
{
    fn prove_universal_circuit(
        &self,
        input: RowsEmbeddedProofInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving universal circuit");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_universal_circuit(
            &input.column_cells,
            &pis.predication_operations,
            &pis.result,
            &input
                .placeholders
                .into(),
            input.is_leaf,
            &pis.bounds,
        )
        .context("while initializing the universal circuit")?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the universal circuit")?;

        let proof_type = "universal_circuit";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "universal circuit size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_full_node(
        &self,
        embedded_tree_proof: Vec<u8>,
        left_child_proof: Vec<u8>,
        right_child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
        is_rows_tree_node: bool,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving full node");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_full_node(
            left_child_proof,
            right_child_proof,
            embedded_tree_proof,
            is_rows_tree_node,
            &pis.bounds,
        )
        .context("while initializating the full node circuit")?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self
            .params
            .generate_proof(input)?;

        let proof_type = "full_node";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "full node size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_partial_node(
        &self,
        input: PartialNodeInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving partial node");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_partial_node(
            input.proven_child_proof,
            embedded_proof,
            input.unproven_child_info,
            input.proven_child_position,
            input.is_rows_tree_node,
            &pis.bounds,
        )
        .context("while initializing the partial node circuit")?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof from the partial node circuit")?;

        let proof_type = "partial_node";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "partial node size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_single_path_leaf(
        &self,
        input: SinglePathLeafInput,
        embedded_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving single path leaf");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_single_path(
            SubProof::new_embedded_tree_proof(embedded_proof)?,
            input.left_child_info,
            input.right_child_info,
            input.node_info,
            input.is_rows_tree_node,
            &pis.bounds,
        )
        .context("while initializing the single path circuit")?;

        let input = QueryCircuitInput::Query(circuit_input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof from the single path circuit")?;

        let proof_type = "single_path_leaf";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "single path leaf size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_single_path_branch(
        &self,
        input: SinglePathBranchInput,
        child_proof: Vec<u8>,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving single path branch");

        let now = std::time::Instant::now();

        let circuit_input = CircuitInput::new_single_path(
            SubProof::new_child_proof(
                child_proof,
                input.child_position,
            )?,
            input.left_child_info,
            input.right_child_info,
            input.node_info,
            input.is_rows_tree_node,
            &pis.bounds,
        )
        .context("while initializing the single path circuit")?;
        let input = QueryCircuitInput::Query(circuit_input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the single path circuit")?;

        let proof_type = "single_path_branch";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "single path branch size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_aggregated_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        query_proof: Vec<u8>,
        indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("proving aggregated revelation");
        let now = std::time::Instant::now();

        let circuit_input = revelation::api::CircuitInput::new_revelation_aggregated(
            query_proof,
            indexing_proof,
            &pis.bounds,
            &placeholders,
            &pis.predication_operations,
            &pis.result,
        )
        .context("while initializing the (empty) revelation circuit")?;

        let input = QueryCircuitInput::Revelation(circuit_input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the (empty) revelation circuit")?;

        let proof_type = "revelation";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "revelation size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_tabular_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        indexing_proof: Vec<u8>,
        matching_rows: Vec<MatchingRow>,
        column_ids: &ColumnIDs,
        limit: u64,
        offset: u64,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("proving tabular revelation");
        let now = std::time::Instant::now();

        let circuit_input = revelation::api::CircuitInput::new_revelation_tabular(
            indexing_proof,
            matching_rows,
            &pis.bounds,
            &placeholders,
            column_ids,
            &pis.predication_operations,
            &pis.result,
            limit,
            offset,
        )
        .context("while initializing the (empty) revelation circuit")?;

        let input = QueryCircuitInput::Revelation(circuit_input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the (empty) revelation circuit")?;

        let proof_type = "revelation";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "revelation size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_non_existence(
        &self,
        input: NonExistenceInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>
    {
        info!("Proving non-existence");

        let now = std::time::Instant::now();

        let primary_column_id = input.column_ids[0];
        let secondary_column_id = input.column_ids[1];
        let rest_column_ids = input.column_ids[2..].to_vec();
        let v_column_ids = ColumnIDs::new(
            primary_column_id,
            secondary_column_id,
            rest_column_ids,
        );

        let placeholders = input
            .placeholders
            .into();
        let query_hashes = QueryHashNonExistenceCircuits::new::<
            MAX_NUM_COLUMNS,
            MAX_NUM_PREDICATE_OPS,
            MAX_NUM_RESULT_OPS,
            MAX_NUM_ITEMS_PER_OUTPUT,
        >(
            &v_column_ids,
            &pis.predication_operations,
            &pis.result,
            &placeholders,
            &pis.bounds,
            input.is_rows_tree_node,
        )?;

        let input = CircuitInput::new_non_existence_input(
            input.node_info,
            input.left_child_info,
            input.right_child_info,
            input.primary_index_value,
            &[
                primary_column_id,
                secondary_column_id,
            ],
            &pis.query_aggregations,
            query_hashes,
            input.is_rows_tree_node,
            &pis.bounds,
            &placeholders,
        )
        .context("while initializing the non-existence circuit")?;

        let input = QueryCircuitInput::Query(input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the non-existence circuit")?;

        let proof_type = "non_existence";
        let time = now
            .elapsed()
            .as_secs_f32();
        info!(
            time,
            proof_type,
            "proof generation time: {:?}",
            now.elapsed()
        );
        histogram!("zkmr_worker_proving_latency", "proof_type" => proof_type).record(time);

        info!(
            "non-existence size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }
}
