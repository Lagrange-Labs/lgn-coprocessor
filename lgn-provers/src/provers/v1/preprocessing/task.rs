use lgn_messages::types::v1::preprocessing::db_tasks::{
    DatabaseType, DbBlockType, DbCellType, DbRowType,
};
use lgn_messages::types::v1::preprocessing::ext_tasks::{
    ExtractionType, FinalExtractionType, MptType,
};
use lgn_messages::types::v1::preprocessing::{db_keys, ext_keys, WorkerTask, WorkerTaskType};
use lgn_messages::types::{
    MessageEnvelope, MessageReplyEnvelope, ReplyType, TaskType, WorkerReply,
};

use crate::provers::v1::preprocessing::prover::{StorageDatabaseProver, StorageExtractionProver};
use crate::provers::LgnProver;

pub struct Preprocessing<P> {
    prover: P,
}

impl<P: StorageExtractionProver + StorageDatabaseProver> LgnProver<TaskType, ReplyType>
    for Preprocessing<P>
{
    fn run(
        &mut self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Preprocessing(task @ WorkerTask { chain_id, .. }) = envelope.inner {
            let key = match &task.task_type {
                WorkerTaskType::Extraction(_) => {
                    let key: ext_keys::ProofKey = (&task).into();
                    key.to_string()
                }
                WorkerTaskType::Database(_) => {
                    let key: db_keys::ProofKey = (&task).into();
                    key.to_string()
                }
            };
            let result = self.run_inner(task)?;
            let reply_type =
                ReplyType::V1Preprocessing(WorkerReply::new(chain_id, Some((key, result))));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }
}
impl<P: StorageExtractionProver + StorageDatabaseProver> Preprocessing<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }

    pub fn run_inner(&mut self, task: WorkerTask) -> anyhow::Result<Vec<u8>> {
        Ok(match task.task_type {
            WorkerTaskType::Extraction(ex) => match ex {
                ExtractionType::MptExtraction(mpt) => match &mpt.mpt_type {
                    MptType::MappingLeaf(input) => self.prover.prove_mapping_variable_leaf(
                        input.key.clone(),
                        input.node.clone(),
                        input.slot,
                        &input.contract_address,
                    )?,
                    MptType::MappingBranch(input) => self.prover.prove_mapping_variable_branch(
                        input.node.clone(),
                        input.children_proofs.to_owned(),
                    )?,
                    MptType::VariableLeaf(input) => self.prover.prove_single_variable_leaf(
                        input.node.clone(),
                        input.slot,
                        &input.contract_address,
                    )?,
                    MptType::VariableBranch(input) => self.prover.prove_single_variable_branch(
                        input.node.clone(),
                        input.children_proofs.clone(),
                    )?,
                },
                ExtractionType::LengthExtraction(length) => {
                    let mut proofs = vec![];
                    for (i, node) in length.nodes.iter().enumerate() {
                        if i == 0 {
                            let proof = self.prover.prove_length_leaf(
                                node.clone(),
                                length.length_slot,
                                length.variable_slot,
                            )?;
                            proofs.push(proof);
                        } else {
                            self.prover.prove_length_branch(
                                node.clone(),
                                proofs.last().unwrap().clone(),
                            )?;
                        }
                    }
                    proofs.last().unwrap().clone()
                }
                ExtractionType::ContractExtraction(contract) => {
                    let mut proofs = vec![];
                    for (i, node) in contract.nodes.iter().enumerate() {
                        if i == 0 {
                            let proof = self.prover.prove_contract_leaf(
                                node.clone(),
                                contract.storage_root.clone(),
                                contract.contract,
                            )?;
                            proofs.push(proof);
                        } else {
                            let proof = self.prover.prove_contract_branch(
                                node.clone(),
                                proofs.last().unwrap().clone(),
                            )?;
                            proofs.push(proof);
                        }
                    }
                    proofs.last().unwrap().clone()
                }
                ExtractionType::BlockExtraction(input) => {
                    self.prover.prove_block(input.rlp_header.to_owned())?
                }
                ExtractionType::FinalExtraction(fe) => match fe.extraction_type {
                    FinalExtractionType::Simple(compound) => {
                        self.prover.prove_final_extraction_simple(
                            fe.block_proof.clone(),
                            fe.contract_proof.clone(),
                            fe.value_proof.clone(),
                            compound,
                        )?
                    }
                    FinalExtractionType::Lengthed => self.prover.prove_final_extraction_lengthed(
                        fe.block_proof.clone(),
                        fe.contract_proof.clone(),
                        fe.value_proof.clone(),
                        fe.length_proof.clone(),
                    )?,
                },
            },
            WorkerTaskType::Database(db) => match db {
                DatabaseType::Cell(cell_type) => match cell_type {
                    DbCellType::Leaf(leaf) => {
                        self.prover.prove_cell_leaf(leaf.identifier, leaf.value)?
                    }
                    DbCellType::Partial(branch) => self.prover.prove_cell_partial(
                        branch.identifier,
                        branch.value,
                        branch.child_proof,
                    )?,
                    DbCellType::Full(full) => self.prover.prove_cell_full(
                        full.identifier,
                        full.value,
                        full.child_proofs,
                    )?,
                },
                DatabaseType::Row(row_type) => match row_type {
                    DbRowType::Leaf(leaf) => {
                        self.prover
                            .prove_row_leaf(leaf.identifier, leaf.value, leaf.cells_proof)?
                    }
                    DbRowType::Partial(partial) => self.prover.prove_row_partial(
                        partial.identifier,
                        partial.value,
                        partial.is_child_left,
                        partial.child_proof,
                        partial.cells_proof,
                    )?,
                    DbRowType::Full(full) => self.prover.prove_row_full(
                        full.identifier,
                        full.value,
                        full.child_proofs,
                        full.cells_proof,
                    )?,
                },
                DatabaseType::Index(block) => {
                    let mut last_proof = None;
                    for input in block.inputs {
                        last_proof = Some(match input {
                            DbBlockType::Leaf(leaf) => self.prover.prove_block_leaf(
                                leaf.block_id,
                                leaf.extraction_proof,
                                leaf.rows_proof,
                            )?,
                            DbBlockType::Parent(parent) => self.prover.prove_block_parent(
                                parent.block_id,
                                parent.old_block_number,
                                parent.old_min,
                                parent.old_max,
                                parent.prev_left_child,
                                parent.prev_right_child,
                                parent.old_rows_tree_hash,
                                parent.extraction_proof,
                                parent.rows_proof,
                            )?,
                            DbBlockType::Membership(membership) => self.prover.prove_membership(
                                membership.block_id,
                                membership.index_value,
                                membership.old_min,
                                membership.old_max,
                                membership.left_child,
                                membership.rows_tree_hash,
                                last_proof.take().unwrap(),
                            )?,
                        });
                    }
                    last_proof.take().unwrap()
                }
            },
        })
    }
}
