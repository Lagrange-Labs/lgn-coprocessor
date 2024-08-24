use crate::provers::v1::query::prover::StorageQueryProver;
use crate::provers::LgnProver;
use anyhow::{bail, Context};
use lgn_messages::types::v1::query::tasks::{
    EmbeddedProofInputType, ProofInputKind, QueryInput, QueryStep,
};
use lgn_messages::types::v1::query::{WorkerTask, WorkerTaskType};
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, WorkerReply,
};
use parsil::assembler::DynamicCircuitPis;
use std::collections::HashMap;
use tracing::info;

pub struct Querying<P> {
    prover: P,
}

impl<P: StorageQueryProver> LgnProver<TaskType, ReplyType> for Querying<P> {
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Query(task @ WorkerTask { chain_id, .. }) = envelope.inner {
            let key = todo!();
            let result = self.run_inner(task)?;
            let reply_type =
                ReplyType::V1Preprocessing(WorkerReply::new(chain_id, Some((key, result))));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }
}

impl<P: StorageQueryProver> Querying<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }

    pub fn run_inner(&mut self, task: WorkerTask) -> anyhow::Result<Vec<u8>> {
        let WorkerTaskType::Query(input) = task.task_type else {
            bail!("Unexpected task type: {:?}", task.task_type);
        };

        let pis: DynamicCircuitPis = bincode::deserialize(&input.pis)?;

        match input.query_step {
            QueryStep::Prepare(parts) => {
                let mut proofs = HashMap::new();

                for part in parts {
                    match (part.embedded_proof_input, part.aggregation_input_kind) {
                        (Some(embedded_input_type), None) => match embedded_input_type {
                            EmbeddedProofInputType::RowsTree(embedded_input) => {
                                let proof =
                                    self.prover.prove_universal_circuit(embedded_input, &pis)?;

                                proofs.insert(part.proof_key, proof);
                            }
                            EmbeddedProofInputType::IndexTree(embedded_input) => {
                                unimplemented!("IndexTree")
                            }
                        },
                        (None, Some(aggregation_input)) => match aggregation_input {
                            ProofInputKind::SinglePathBranch(sb) => {
                                let child_proof = proofs
                                    .remove(&sb.proven_child_location)
                                    .context("Missing proof")?;

                                let proof =
                                    self.prover
                                        .prove_single_path_branch(sb, child_proof, &pis)?;
                                proofs.insert(part.proof_key, proof);
                            }
                            _ => {
                                bail!("Invalid inputs")
                            }
                        },
                        (Some(embedded_input_type), Some(aggregation_input)) => {
                            let embedded_proof = match embedded_input_type {
                                EmbeddedProofInputType::RowsTree(embedded_input) => {
                                    self.prover.prove_universal_circuit(embedded_input, &pis)?
                                }
                                EmbeddedProofInputType::IndexTree(embedded_input) => {
                                    unimplemented!("IndexTree")
                                }
                            };

                            match aggregation_input {
                                ProofInputKind::SinglePathLeaf(sp) => {
                                    let proof = self.prover.prove_single_path_leaf(
                                        sp,
                                        embedded_proof,
                                        &pis,
                                    )?;
                                    proofs.insert(part.proof_key, proof);
                                }
                                ProofInputKind::PartialNode(mut sp) => {
                                    if sp.proven_child_proof.is_empty() {
                                        sp.proven_child_proof = proofs
                                            .remove(&sp.proven_child_proof_location)
                                            .context("Missing proof")?;
                                    }
                                    let proof =
                                        self.prover.prove_partial_node(sp, embedded_proof, &pis)?;
                                    proofs.insert(part.proof_key, proof);
                                }
                                ProofInputKind::FullNode(mut f) => {
                                    if f.left_child_proof.is_empty() {
                                        f.left_child_proof = proofs
                                            .remove(&f.left_child_proof_location)
                                            .context("Missing proof")?;
                                    }

                                    if f.right_child_proof.is_empty() {
                                        f.right_child_proof = proofs
                                            .remove(&f.right_child_proof_location)
                                            .context("Missing proof")?;
                                    }

                                    let proof = self.prover.prove_full_node(
                                        embedded_proof,
                                        f.left_child_proof,
                                        f.right_child_proof,
                                        &pis,
                                        f.is_rows_tree_node,
                                    )?;
                                    proofs.insert(part.proof_key, proof);
                                }
                                _ => {
                                    bail!("Invalid inputs")
                                }
                            }
                        }
                        (None, None) => {
                            bail!("Invalid inputs")
                        }
                    }
                }
            }
            QueryStep::Revelation(rev) => {
                let query_proof = rev.query_proof;
                let indexing_proof = rev.indexing_proof;

                return self.prover.prove_revelation(
                    &pis,
                    rev.placeholders,
                    query_proof,
                    indexing_proof,
                );
            }
        }

        Ok(vec![])
    }
}
