use std::collections::HashMap;

use anyhow::Context;
use lgn_messages::types::v1::query::tasks::MatchingRowInput;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::RowsChunkInput;
use lgn_messages::types::v1::query::NUM_CHUNKS;
use lgn_messages::types::v1::query::NUM_ROWS;
use parsil::assembler::DynamicCircuitPis;
use verifiable_db::api::QueryCircuitInput;
use verifiable_db::api::QueryParameters;
use verifiable_db::query::api::CircuitInput;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation;
use verifiable_db::revelation::api::MatchingRow;

use super::INDEX_TREE_MAX_DEPTH;
use super::MAX_NUM_COLUMNS;
use super::MAX_NUM_ITEMS_PER_OUTPUT;
use super::MAX_NUM_OUTPUTS;
use super::MAX_NUM_PLACEHOLDERS;
use super::MAX_NUM_PREDICATE_OPS;
use super::MAX_NUM_RESULT_OPS;
use super::ROW_TREE_MAX_DEPTH;
use crate::params;

pub(crate) struct QueryEuclidProver {
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

impl QueryEuclidProver {
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
        Self { params }
    }

    pub(crate) async fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> anyhow::Result<Self> {
        let params = params::download_and_checksum(url, dir, file, checksums).await?;
        let params = tokio::task::spawn_blocking(move || {
            let reader = std::io::BufReader::new(params.as_ref());
            bincode::deserialize_from(reader)
        })
        .await??;
        Ok(Self { params })
    }

    pub(super) fn prove_universal_circuit(
        &self,
        input: MatchingRowInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        let circuit_input = CircuitInput::new_universal_circuit(
            &input.column_cells,
            &pis.predication_operations,
            &pis.result,
            &input.placeholders.into(),
            input.is_leaf,
            &pis.bounds,
        )
        .context("while initializing the universal circuit")?;

        let input = QueryCircuitInput::Query(circuit_input);
        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the universal circuit")?;

        Ok(proof)
    }

    pub(super) fn prove_row_chunks(
        &self,
        input: RowsChunkInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        let placeholders = input.placeholders.into();

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

        Ok(proof)
    }

    pub(super) fn prove_chunk_aggregation(
        &self,
        chunks_proofs: &[Vec<u8>],
    ) -> anyhow::Result<Vec<u8>> {
        let input = CircuitInput::new_chunk_aggregation_input(chunks_proofs)
            .context("while initializing the chunk-aggregation circuit")?;

        let input = QueryCircuitInput::Query(input);

        let proof = self
            .params
            .generate_proof(input)
            .context("while generating proof for the chunk-aggregation circuit")?;

        Ok(proof)
    }

    pub(super) fn prove_non_existence(
        &self,
        input: NonExistenceInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        let placeholders = input.placeholders.into();

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

        Ok(proof)
    }

    pub(super) fn prove_aggregated_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        query_proof: Vec<u8>,
        indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
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

        Ok(proof)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prove_tabular_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        indexing_proof: Vec<u8>,
        matching_rows: Vec<MatchingRow>,
        column_ids: &ColumnIDs,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<u8>> {
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

        Ok(proof)
    }
}
