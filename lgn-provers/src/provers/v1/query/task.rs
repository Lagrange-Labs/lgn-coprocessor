use anyhow::bail;
use lgn_messages::types::v1::query::keys::ProofKey;
use lgn_messages::types::v1::query::tasks::Hydratable;
use lgn_messages::types::v1::query::tasks::HydratableMatchingRow;
use lgn_messages::types::v1::query::tasks::ProofInputKind;
use lgn_messages::types::v1::query::tasks::QueryStep;
use lgn_messages::types::v1::query::tasks::RevelationInput;
use lgn_messages::types::v1::query::WorkerTask;
use lgn_messages::types::v1::query::WorkerTaskType;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;
use parsil::assembler::DynamicCircuitPis;

use super::euclid_prover::QueryEuclidProver;
use crate::provers::LgnProver;

impl LgnProver for QueryEuclidProver {
    fn run(
        &self,
        envelope: MessageEnvelope,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();

        if let TaskType::V1Query(ref task @ WorkerTask { chain_id, .. }) = envelope.inner {
            let key: ProofKey = task.into();
            let result = self.run_inner(task)?;
            let reply_type = ReplyType::V1Query(WorkerReply::new(
                chain_id,
                Some((key.to_string(), result)),
                ProofCategory::Querying,
            ));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            bail!("Unexpected task: {:?}", envelope);
        }
    }
}

impl QueryEuclidProver {
    pub fn run_inner(
        &self,
        task: &WorkerTask,
    ) -> anyhow::Result<Vec<u8>> {
        let WorkerTaskType::Query(ref input) = task.task_type;

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
                    bail!("Wrong RevelationInput for QueryStep::Tabular");
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
