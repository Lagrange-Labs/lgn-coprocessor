use anyhow::Context;
use lgn_messages::types::v1::query::tasks::MatchingRowInput;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::RowsChunkInput;
use lgn_messages::types::v1::query::NUM_CHUNKS;
use lgn_messages::types::v1::query::NUM_ROWS;
use metrics::histogram;
use parsil::assembler::DynamicCircuitPis;
use tracing::debug;
use tracing::info;
use verifiable_db::api::QueryCircuitInput;
use verifiable_db::api::QueryParameters;
use verifiable_db::query::api::CircuitInput;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation;
use verifiable_db::revelation::api::MatchingRow;

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

pub(crate) struct EuclidQueryProver {
    params: QueryParameters<
        NUM_CHUNKS,
        NUM_ROWS,
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

impl EuclidQueryProver {
    #[allow(dead_code)]
    pub fn new(
        params: QueryParameters<
            NUM_CHUNKS,
            NUM_ROWS,
            ROW_TREE_MAX_DEPTH,
            INDEX_TREE_MAX_DEPTH,
            MAX_NUM_COLUMNS,
            MAX_NUM_PREDICATE_OPS,
            MAX_NUM_RESULT_OPS,
            MAX_NUM_OUTPUTS,
            MAX_NUM_ITEMS_PER_OUTPUT,
            MAX_NUM_PLACEHOLDERS,
        >
    ) -> Self {
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
    ) -> anyhow::Result<Self> {
        let params = ParamsLoader::prepare_raw(
            url,
            dir,
            file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )
        .context("while loading bincode-serialized parameters")?;
        let reader = std::io::BufReader::new(params.as_ref());
        let params = bincode::deserialize_from(reader)?;
        Ok(
            Self {
                params,
            },
        )
    }
}

impl StorageQueryProver for EuclidQueryProver {
    fn prove_universal_circuit(
        &self,
        input: MatchingRowInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving universal circuit");

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

        debug!(
            "universal circuit size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_row_chunks(
        &self,
        input: RowsChunkInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row-chunks");

        let now = std::time::Instant::now();

        let placeholders = input
            .placeholders
            .into();

        let input = CircuitInput::new_row_chunks_input(
            &input.rows,
            &pis.predication_operations,
            &placeholders,
            &pis.bounds,
            &pis.result,
        )
        .context("while initializing the rows-chunk circuit")?;

        let input = QueryCircuitInput::Query(input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the rows-chunk circuit")?;

        let proof_type = "rows_chunk";
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

        debug!(
            "rows-chunk size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_chunk_aggregation(
        &self,
        chunks_proofs: &[Vec<u8>],
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving row-chunks");

        let now = std::time::Instant::now();

        let input = CircuitInput::new_chunk_aggregation_input(chunks_proofs)
            .context("while initializing the chunk-aggregation circuit")?;

        let input = QueryCircuitInput::Query(input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the chunk-aggregation circuit")?;

        let proof_type = "chunk_aggregation";
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

        debug!(
            "chunk-aggregation size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_non_existence(
        &self,
        input: NonExistenceInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving non-existence");

        let now = std::time::Instant::now();

        let placeholders = input
            .placeholders
            .into();

        let input = CircuitInput::new_non_existence_input(
            input.index_path,
            &input.column_ids,
            &pis.predication_operations,
            &pis.result,
            &placeholders,
            &pis.bounds,
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

        debug!(
            "non-existence size in kB: {}",
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
    ) -> anyhow::Result<Vec<u8>> {
        debug!("proving aggregated revelation");
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

        debug!(
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
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("proving tabular revelation");
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

        debug!(
            "revelation size in kB: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }
}
