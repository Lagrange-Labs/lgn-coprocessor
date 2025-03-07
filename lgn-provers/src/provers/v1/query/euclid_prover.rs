use std::collections::HashMap;

use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::v1::query::tasks::Hydratable;
use lgn_messages::types::v1::query::tasks::HydratableMatchingRow;
use lgn_messages::types::v1::query::tasks::MatchingRowInput;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::ProofInputKind;
use lgn_messages::types::v1::query::tasks::QueryStep;
use lgn_messages::types::v1::query::tasks::RevelationInput;
use lgn_messages::types::v1::query::tasks::RowsChunkInput;
use lgn_messages::types::v1::query::WorkerTaskType;
use lgn_messages::types::v1::query::NUM_CHUNKS;
use lgn_messages::types::v1::query::NUM_ROWS;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::TaskType;
use lgn_messages::Proof;
use metrics::histogram;
use parsil::assembler::DynamicCircuitPis;
use tracing::info;
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
use crate::provers::LgnProver;

pub type ConcreteCircuitInput = QueryCircuitInput<
    NUM_CHUNKS,
    NUM_ROWS,
    ROW_TREE_MAX_DEPTH,
    INDEX_TREE_MAX_DEPTH,
    MAX_NUM_COLUMNS,
    MAX_NUM_PREDICATE_OPS,
    MAX_NUM_PREDICATE_OPS,
    MAX_NUM_OUTPUTS,
    MAX_NUM_ITEMS_PER_OUTPUT,
    MAX_NUM_PLACEHOLDERS,
>;

pub type ConcreteQueryParameters = QueryParameters<
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
>;

pub struct EuclidQueryProver {
    params: ConcreteQueryParameters,
}

impl EuclidQueryProver {
    pub fn new(params: ConcreteQueryParameters) -> Self {
        Self { params }
    }

    pub fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> anyhow::Result<Self> {
        let params = params::prepare_raw(url, dir, file, checksums)
            .context("while loading bincode-serialized parameters")?;
        let reader = std::io::BufReader::new(params.as_ref());
        let params = bincode::deserialize_from(reader)?;
        Ok(Self { params })
    }
}

impl EuclidQueryProver {
    fn prove_circuit_input(
        &self,
        circuit_input: ConcreteCircuitInput,
    ) -> anyhow::Result<Proof> {
        info!("Proving query circuit");
        let now = std::time::Instant::now();

        let proof = self
            .params
            .generate_proof(circuit_input)
            .context("while generating proof for the universal circuit")?;

        let time = now.elapsed().as_secs_f32();
        histogram!("zkmr_worker_proving_latency", "proof_type" => "circuit_input").record(time);

        info!(
            "Query circuit. proof_size_kb: {} time: {:?}",
            proof.len() / 1024,
            now.elapsed()
        );

        Ok(proof)
    }

    fn prove_universal_circuit(
        &self,
        input: MatchingRowInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Proof> {
        let circuit_input = CircuitInput::new_universal_circuit(
            &input.column_cells,
            &pis.predication_operations,
            &pis.result,
            &input.placeholders.into(),
            input.is_leaf,
            &pis.bounds,
        )
        .context("while initializing the universal circuit")?;

        let proof = self.prove_circuit_input(QueryCircuitInput::Query(circuit_input))?;

        Ok(proof)
    }

    fn prove_row_chunks(
        &self,
        input: RowsChunkInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Proof> {
        let placeholders = input.placeholders.into();
        let input = CircuitInput::new_row_chunks_input(
            &input.rows,
            &pis.predication_operations,
            &placeholders,
            &pis.bounds,
            &pis.result,
        )
        .context("while initializing the rows-chunk circuit")?;

        let proof = self.prove_circuit_input(QueryCircuitInput::Query(input))?;

        Ok(proof)
    }

    fn prove_chunk_aggregation(
        &self,
        chunks_proofs: &[Vec<u8>],
    ) -> anyhow::Result<Proof> {
        let input = CircuitInput::new_chunk_aggregation_input(chunks_proofs)
            .context("while initializing the chunk-aggregation circuit")?;

        let proof = self.prove_circuit_input(QueryCircuitInput::Query(input))?;

        Ok(proof)
    }

    fn prove_non_existence(
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

        let proof = self.prove_circuit_input(QueryCircuitInput::Query(input))?;

        Ok(proof)
    }

    fn prove_aggregated_revelation(
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

        let proof = self.prove_circuit_input(QueryCircuitInput::Revelation(circuit_input))?;

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

        let proof = self.prove_circuit_input(QueryCircuitInput::Revelation(circuit_input))?;

        Ok(proof)
    }

    pub fn run_inner(
        &self,
        task_type: &WorkerTaskType,
    ) -> anyhow::Result<Vec<u8>> {
        let WorkerTaskType::Query(ref input) = task_type;

        let pis: DynamicCircuitPis = serde_json::from_slice(&input.pis)?;

        let final_proof = match &input.query_step {
            QueryStep::Tabular(rows_inputs, revelation_input) => {
                let RevelationInput::Tabular {
                    placeholders,
                    indexing_proof,
                    matching_rows,
                    column_ids,
                    limit,
                    offset,
                    ..
                } = revelation_input
                else {
                    panic!("Wrong RevelationInput for QueryStep::Tabular");
                };

                let mut matching_rows_proofs = vec![];
                for (row_input, mut matching_row) in rows_inputs.iter().zip(matching_rows.clone()) {
                    let proof = self.prove_universal_circuit(row_input.clone(), &pis)?;

                    if let Hydratable::Dehydrated(_) = &matching_row.proof {
                        matching_row.proof.hydrate(proof);
                    }

                    let matching_row_proof = HydratableMatchingRow::into_matching_row(matching_row);
                    matching_rows_proofs.push(matching_row_proof);
                }

                self.prove_tabular_revelation(
                    &pis,
                    placeholders.clone().into(),
                    indexing_proof.clone_proof(),
                    matching_rows_proofs,
                    column_ids,
                    *limit,
                    *offset,
                )?
            },
            QueryStep::Aggregation(input) => {
                match &input.input_kind {
                    ProofInputKind::RowsChunk(rc) => self.prove_row_chunks(rc.clone(), &pis),
                    ProofInputKind::ChunkAggregation(ca) => {
                        let chunks_proofs = ca
                            .child_proofs
                            .iter()
                            .map(|proof| proof.clone_proof())
                            .collect::<Vec<_>>();
                        self.prove_chunk_aggregation(&chunks_proofs)
                    },
                    ProofInputKind::NonExistence(ne) => self.prove_non_existence(*ne.clone(), &pis),
                }?
            },
            QueryStep::Revelation(input) => {
                match input {
                    RevelationInput::Aggregated {
                        placeholders,
                        indexing_proof,
                        query_proof,
                        ..
                    } => {
                        self.prove_aggregated_revelation(
                            &pis,
                            placeholders.clone().into(),
                            query_proof.clone_proof(),
                            indexing_proof.clone_proof(),
                        )
                    },
                    RevelationInput::Tabular {
                        placeholders,
                        indexing_proof,
                        matching_rows,
                        column_ids,
                        limit,
                        offset,
                        ..
                    } => {
                        self.prove_tabular_revelation(
                            &pis,
                            placeholders.clone().into(),
                            indexing_proof.clone_proof(),
                            matching_rows
                                .iter()
                                .cloned()
                                .map(HydratableMatchingRow::into_matching_row)
                                .collect(),
                            column_ids,
                            *limit,
                            *offset,
                        )
                    },
                }?
            },
        };

        Ok(final_proof)
    }
}

impl LgnProver for EuclidQueryProver {
    fn run(
        &self,
        envelope: lgn_messages::types::MessageEnvelope,
    ) -> anyhow::Result<lgn_messages::types::MessageReplyEnvelope> {
        let task_id = envelope.task_id.clone();

        match envelope.task() {
            TaskType::V1Preprocessing(..) => {
                bail!(
                "EuclidQueryProver: unsupported task type. task_type: V1Preprocessing task_id: {}",
                task_id,
            )
            },
            TaskType::V1Query(task_type) => {
                let proof = self.run_inner(task_type)?;
                Ok(MessageReplyEnvelope::new(task_id, proof))
            },
            TaskType::V1Groth16(..) => {
                bail!(
                    "EuclidQueryProver: unsupported task type. task_type: V1Groth16 task_id: {}",
                    task_id,
                )
            },
        }
    }
}
