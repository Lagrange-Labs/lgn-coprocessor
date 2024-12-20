use std::collections::HashMap;

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

use crate::provers::v1::query::prover::StorageQueryProver;
use crate::provers::LgnProver;

pub struct Querying<P>
{
    prover: P,
}

impl<P: StorageQueryProver> LgnProver<TaskType, ReplyType> for Querying<P>
{
    fn run(
        &self,
        envelope: &MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>>
    {
        let query_id = envelope
            .query_id
            .clone();
        let task_id = envelope
            .task_id
            .clone();

        if let TaskType::V1Query(
            ref task @ WorkerTask {
                chain_id,
                ..
            },
        ) = envelope.inner
        {
            let key: ProofKey = task.into();
            let result = self.run_inner(task)?;
            let reply_type = ReplyType::V1Query(
                WorkerReply::new(
                    chain_id,
                    Some(
                        (
                            key.to_string(),
                            result,
                        ),
                    ),
                    ProofCategory::Querying,
                ),
            );
            Ok(
                MessageReplyEnvelope::new(
                    query_id,
                    task_id,
                    reply_type,
                ),
            )
        }
        else
        {
            bail!(
                "Received unexpected task: {:?}",
                envelope
            );
        }
    }
}

impl<P: StorageQueryProver> Querying<P>
{
    pub fn new(prover: P) -> Self
    {
        Self {
            prover,
        }
    }

    pub fn run_inner(
        &self,
        task: &WorkerTask,
    ) -> anyhow::Result<Vec<u8>>
    {
        #[allow(irrefutable_let_patterns)]
        let WorkerTaskType::Query(ref input) = task.task_type
        else
        {
            bail!(
                "Unexpected task type: {:?}",
                task.task_type
            );
        };

        let pis: DynamicCircuitPis = serde_json::from_slice(&input.pis)?;

        let mut proofs = HashMap::new();

        match &input.query_step
        {
            QueryStep::Tabular(rows_inputs, revelation_input) =>
            {
                for input in rows_inputs
                {
                    let proof_key = input
                        .proof_key
                        .clone();
                    let proof = self
                        .prover
                        .prove_universal_circuit(
                            input.clone(),
                            &pis,
                        )?;
                    proofs.insert(
                        proof_key,
                        proof,
                    );
                }

                match revelation_input
                {
                    RevelationInput::Tabular {
                        placeholders,
                        indexing_proof,
                        matching_rows,
                        column_ids,
                        limit,
                        offset,
                        ..
                    } =>
                    {
                        let matching_rows = matching_rows
                            .iter()
                            .cloned()
                            .map(
                                |mut row| {
                                    if let Hydratable::Dehydrated(key) = &row.proof
                                    {
                                        row.proof
                                            .hydrate(
                                                proofs
                                                    .get(key)
                                                    .unwrap_or_else(|| panic!("Cannot find matching-row proof: {key:?}")).clone(),
                                            );
                                    }

                                    HydratableMatchingRow::into_matching_row(row)
                                },
                            )
                            .collect();

                        return self
                            .prover
                            .prove_tabular_revelation(
                                &pis,
                                placeholders
                                    .clone()
                                    .into(),
                                indexing_proof.clone_proof(),
                                matching_rows,
                                column_ids,
                                *limit,
                                *offset,
                            );
                    },
                    _ => panic!("Wrong RevelationInput for QueryStep::Tabular"),
                }
            },
            QueryStep::Aggregation(inputs) =>
            {
                for input in inputs
                {
                    let proof_key = input
                        .proof_key
                        .clone();
                    match &input.input_kind
                    {
                        ProofInputKind::RowsChunk(rc) =>
                        {
                            let proof = self
                                .prover
                                .prove_row_chunks(
                                    rc.clone(),
                                    &pis,
                                )?;
                            proofs.insert(
                                proof_key.to_owned(),
                                proof,
                            );
                        },
                        ProofInputKind::ChunkAggregation(ca) =>
                        {
                            let chunks_proofs = ca
                                .child_proofs
                                .iter()
                                .map(
                                    |proof| {
                                        match proof
                                        {
                                            Hydratable::Hydrated(_) => proof.clone_proof(),
                                            Hydratable::Dehydrated(key) => proofs
                                                .remove(key)
                                                .expect(
                                                    "Cannot find rows-chunk proof: {proof_key:?}",
                                                ),
                                        }
                                    },
                                )
                                .collect::<Vec<_>>();
                            let proof = self
                                .prover
                                .prove_chunk_aggregation(&chunks_proofs)?;
                            proofs.insert(
                                proof_key.to_owned(),
                                proof,
                            );
                        },
                        ProofInputKind::NonExistence(ne) =>
                        {
                            let proof = self
                                .prover
                                .prove_non_existence(
                                    *ne.clone(),
                                    &pis,
                                )?;
                            proofs.insert(
                                proof_key.to_owned(),
                                proof,
                            );
                        },
                    }
                }
            },
            QueryStep::Revelation(input) =>
            {
                match input
                {
                    RevelationInput::Aggregated {
                        placeholders,
                        indexing_proof,
                        query_proof,
                        ..
                    } =>
                    {
                        return self
                            .prover
                            .prove_aggregated_revelation(
                                &pis,
                                placeholders
                                    .clone()
                                    .into(),
                                query_proof.clone_proof(),
                                indexing_proof.clone_proof(),
                            );
                    },
                    RevelationInput::Tabular {
                        placeholders,
                        indexing_proof,
                        matching_rows,
                        column_ids,
                        limit,
                        offset,
                        ..
                    } =>
                    {
                        return self
                            .prover
                            .prove_tabular_revelation(
                                &pis,
                                placeholders
                                    .clone()
                                    .into(),
                                indexing_proof.clone_proof(),
                                matching_rows
                                    .iter()
                                    .cloned()
                                    .map(HydratableMatchingRow::into_matching_row)
                                    .collect(),
                                column_ids,
                                *limit,
                                *offset,
                            );
                    },
                }
            },
        }

        // Only one proof should be left
        if proofs.len() > 1
        {
            bail!(
                "Invalid number of proofs left: {}",
                proofs.len()
            );
        }

        let final_proof = proofs
            .values()
            .next()
            .unwrap()
            .clone();
        Ok(final_proof)
    }
}
