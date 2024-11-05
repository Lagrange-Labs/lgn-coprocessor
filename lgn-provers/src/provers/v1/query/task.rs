use std::collections::HashMap;

use anyhow::bail;
use lgn_messages::types::v1::query::keys::ProofKey;
use lgn_messages::types::v1::query::tasks::EmbeddedProofInputType;
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
            QueryStep::Prepare(ref parts) =>
            {
                for part in parts
                {
                    match (
                        &part.embedded_proof_input,
                        &part.aggregation_input_kind,
                    )
                    {
                        (Some(embedded_input_type), None) =>
                        {
                            match embedded_input_type
                            {
                                EmbeddedProofInputType::RowsTree(embedded_input) =>
                                {
                                    let proof = self
                                        .prover
                                        .prove_universal_circuit(
                                            embedded_input.to_owned(),
                                            &pis,
                                        )?;

                                    proofs.insert(
                                        part.proof_key
                                            .to_owned(),
                                        proof,
                                    );
                                },
                                EmbeddedProofInputType::IndexTree(_) =>
                                {
                                    bail!("IndexTree always must have aggregation input")
                                },
                            }
                        },
                        (None, Some(aggregation_input)) =>
                        {
                            match &aggregation_input
                            {
                                ProofInputKind::SinglePathBranch(sb) =>
                                {
                                    let child_proof = proofs
                                        .remove(&sb.proven_child_location)
                                        .unwrap_or(
                                            sb.proven_child_proof
                                                .to_owned(),
                                        );

                                    let proof = self
                                        .prover
                                        .prove_single_path_branch(
                                            sb.to_owned(),
                                            child_proof,
                                            &pis,
                                        )?;
                                    proofs.insert(
                                        part.proof_key
                                            .to_owned(),
                                        proof,
                                    );
                                },
                                ProofInputKind::NonExistence(ne) =>
                                {
                                    let proof = self
                                        .prover
                                        .prove_non_existence(
                                            ne.to_owned(),
                                            &pis,
                                        )?;
                                    proofs.insert(
                                        part.proof_key
                                            .to_owned(),
                                        proof,
                                    );
                                },
                                _ =>
                                {},
                            }
                        },
                        (Some(embedded_input_type), Some(ref aggregation_input)) =>
                        {
                            let embedded_proof = match embedded_input_type
                            {
                                EmbeddedProofInputType::RowsTree(embedded_input) =>
                                {
                                    self.prover
                                        .prove_universal_circuit(
                                            embedded_input.to_owned(),
                                            &pis,
                                        )?
                                },
                                EmbeddedProofInputType::IndexTree(embedded_input) =>
                                {
                                    embedded_input
                                        .rows_proof
                                        .to_owned()
                                },
                            };

                            match aggregation_input
                            {
                                ProofInputKind::SinglePathLeaf(sp) =>
                                {
                                    let proof = self
                                        .prover
                                        .prove_single_path_leaf(
                                            sp.to_owned(),
                                            embedded_proof,
                                            &pis,
                                        )?;
                                    proofs.insert(
                                        part.proof_key
                                            .to_owned(),
                                        proof,
                                    );
                                },
                                ProofInputKind::PartialNode(sp) =>
                                {
                                    let mut sp = sp.clone();
                                    if sp
                                        .proven_child_proof
                                        .is_empty()
                                    {
                                        sp.proven_child_proof = proofs
                                            .remove(&sp.proven_child_proof_location)
                                            .unwrap_or(
                                                sp.proven_child_proof
                                                    .to_owned(),
                                            );
                                    }
                                    let proof = self
                                        .prover
                                        .prove_partial_node(
                                            sp,
                                            embedded_proof,
                                            &pis,
                                        )?;
                                    proofs.insert(
                                        part.proof_key
                                            .to_owned(),
                                        proof,
                                    );
                                },
                                ProofInputKind::FullNode(f) =>
                                {
                                    let mut f = f.clone();
                                    if f.left_child_proof
                                        .is_empty()
                                    {
                                        f.left_child_proof = proofs
                                            .remove(&f.left_child_proof_location)
                                            .unwrap_or(f.left_child_proof);
                                    }

                                    if f.right_child_proof
                                        .is_empty()
                                    {
                                        f.right_child_proof = proofs
                                            .remove(&f.right_child_proof_location)
                                            .unwrap_or(f.right_child_proof);
                                    }

                                    let proof = self
                                        .prover
                                        .prove_full_node(
                                            embedded_proof,
                                            f.left_child_proof,
                                            f.right_child_proof,
                                            &pis,
                                            f.is_rows_tree_node,
                                        )?;
                                    proofs.insert(
                                        part.proof_key
                                            .clone(),
                                        proof,
                                    );
                                },
                                _ =>
                                {
                                    bail!("Invalid inputs")
                                },
                            }
                        },
                        (None, None) =>
                        {
                            bail!("Invalid inputs")
                        },
                    }
                }
            },
            QueryStep::Revelation(rev) =>
            {
                match rev
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
                                query_proof.to_owned(),
                                indexing_proof.to_owned(),
                            );
                    },
                    RevelationInput::Tabular {
                        placeholders,
                        indexing_proof: preprocessing_proof,
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
                                preprocessing_proof
                                    .clone()
                                    .into_proof(),
                                matching_rows
                                    .into_iter()
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
