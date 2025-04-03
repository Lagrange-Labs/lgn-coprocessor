use lgn_messages::types::v1::preprocessing::db_keys;
use lgn_messages::types::v1::preprocessing::db_tasks::DatabaseType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbBlockType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbCellType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbRowType;
use lgn_messages::types::v1::preprocessing::ext_keys;
use lgn_messages::types::v1::preprocessing::ext_tasks::ExtractionType;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtraction;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtractionType;
use lgn_messages::types::v1::preprocessing::ext_tasks::MptType;
use lgn_messages::types::v1::preprocessing::WorkerTask;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;

use super::euclid_prover::EuclidProver;
use crate::provers::LgnProver;

impl LgnProver for EuclidProver {
    fn run(
        &self,
        envelope: MessageEnvelope,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();
        if let TaskType::V1Preprocessing(task @ WorkerTask { chain_id, .. }) = envelope.inner {
            let key = match &task.task_type {
                WorkerTaskType::Extraction(_) => {
                    let key: ext_keys::ProofKey = (&task).into();
                    key.to_string()
                },
                WorkerTaskType::Database(_) => {
                    let key: db_keys::ProofKey = (&task).into();
                    key.to_string()
                },
            };
            let result = self.run_inner(task)?;
            let reply_type = ReplyType::V1Preprocessing(WorkerReply::new(
                chain_id,
                Some((key, result)),
                ProofCategory::Querying,
            ));
            Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
        } else {
            anyhow::bail!("Received unexpected task: {:?}", envelope);
        }
    }
}

impl EuclidProver {
    pub fn run_inner(
        &self,
        task: WorkerTask,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(match task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt) => {
                        match &mpt.mpt_type {
                            MptType::VariableLeaf(variable_leaf) => {
                                self.prove_single_variable_leaf(
                                    variable_leaf.node.clone(),
                                    variable_leaf.slot,
                                    variable_leaf.column_id,
                                )?
                            },
                            MptType::MappingLeaf(mapping_leaf) => {
                                self.prove_mapping_variable_leaf(
                                    mapping_leaf.key.clone(),
                                    mapping_leaf.node.clone(),
                                    mapping_leaf.slot,
                                    mapping_leaf.key_id,
                                    mapping_leaf.value_id,
                                )?
                            },
                            MptType::MappingBranch(mapping_branch) => {
                                self.prove_mapping_variable_branch(
                                    mapping_branch.node.clone(),
                                    mapping_branch.children_proofs.to_owned(),
                                )?
                            },
                            MptType::VariableBranch(variable_branch) => {
                                self.prove_single_variable_branch(
                                    variable_branch.node.clone(),
                                    variable_branch.children_proofs.clone(),
                                )?
                            },
                        }
                    },
                    ExtractionType::LengthExtraction(length) => {
                        let mut proofs = vec![];
                        for (i, node) in length.nodes.iter().enumerate() {
                            if i == 0 {
                                let proof = self.prove_length_leaf(
                                    node.clone(),
                                    length.length_slot,
                                    length.variable_slot,
                                )?;
                                proofs.push(proof);
                            } else {
                                self.prove_length_branch(
                                    node.clone(),
                                    proofs.last().unwrap().clone(),
                                )?;
                            }
                        }
                        proofs.last().unwrap().clone()
                    },
                    ExtractionType::ContractExtraction(contract) => {
                        let mut proofs = vec![];
                        for (i, node) in contract.nodes.iter().enumerate() {
                            if i == 0 {
                                let proof = self.prove_contract_leaf(
                                    node.clone(),
                                    contract.storage_root.clone(),
                                    contract.contract,
                                )?;
                                proofs.push(proof);
                            } else {
                                let proof = self.prove_contract_branch(
                                    node.clone(),
                                    proofs.last().unwrap().clone(),
                                )?;
                                proofs.push(proof);
                            }
                        }
                        proofs.last().unwrap().clone()
                    },
                    ExtractionType::BlockExtraction(block) => {
                        self.prove_block(block.rlp_header.to_owned())?
                    },
                    ExtractionType::FinalExtraction(final_extraction) => {
                        match *final_extraction {
                            FinalExtraction::Single(single_table_extraction) => {
                                match single_table_extraction.extraction_type {
                                    FinalExtractionType::Simple(compound) => {
                                        self.prove_final_extraction_simple(
                                            single_table_extraction.block_proof.clone(),
                                            single_table_extraction.contract_proof.clone(),
                                            single_table_extraction.value_proof.clone(),
                                            compound,
                                        )?
                                    },
                                    FinalExtractionType::Lengthed => {
                                        self.prove_final_extraction_lengthed(
                                            single_table_extraction.block_proof.clone(),
                                            single_table_extraction.contract_proof.clone(),
                                            single_table_extraction.value_proof.clone(),
                                            single_table_extraction.length_proof.clone(),
                                        )?
                                    },
                                }
                            },
                            FinalExtraction::Merge(mapping_table_extraction) => {
                                self.prove_final_extraction_merge(
                                    mapping_table_extraction.block_proof.clone(),
                                    mapping_table_extraction.contract_proof.clone(),
                                    mapping_table_extraction.simple_table_proof.clone(),
                                    mapping_table_extraction.mapping_table_proof.clone(),
                                )?
                            },
                        }
                    },
                }
            },
            WorkerTaskType::Database(db) => {
                match db {
                    DatabaseType::Cell(cell_type) => {
                        match cell_type {
                            DbCellType::Leaf(leaf) => {
                                self.prove_cell_leaf(
                                    leaf.identifier,
                                    leaf.value,
                                    leaf.is_multiplier,
                                )?
                            },
                            DbCellType::Partial(branch) => {
                                self.prove_cell_partial(
                                    branch.identifier,
                                    branch.value,
                                    branch.is_multiplier,
                                    branch.child_proof,
                                )?
                            },
                            DbCellType::Full(full) => {
                                self.prove_cell_full(
                                    full.identifier,
                                    full.value,
                                    full.is_multiplier,
                                    full.child_proofs,
                                )?
                            },
                        }
                    },
                    DatabaseType::Row(row_type) => {
                        match row_type {
                            DbRowType::Leaf(leaf) => {
                                self.prove_row_leaf(
                                    leaf.identifier,
                                    leaf.value,
                                    leaf.is_multiplier,
                                    leaf.cells_proof,
                                )?
                            },
                            DbRowType::Partial(partial) => {
                                self.prove_row_partial(
                                    partial.identifier,
                                    partial.value,
                                    partial.is_multiplier,
                                    partial.is_child_left,
                                    partial.child_proof.to_owned(),
                                    partial.cells_proof.to_owned(),
                                )?
                            },
                            DbRowType::Full(full) => {
                                self.prove_row_full(
                                    full.identifier,
                                    full.value,
                                    full.is_multiplier,
                                    full.child_proofs,
                                    full.cells_proof,
                                )?
                            },
                        }
                    },
                    DatabaseType::Index(block) => {
                        let mut last_proof = None;
                        for input in &block.inputs {
                            last_proof = Some(match input {
                                DbBlockType::Leaf(leaf) => {
                                    self.prove_block_leaf(
                                        leaf.block_id,
                                        leaf.extraction_proof.to_owned(),
                                        leaf.rows_proof.to_owned(),
                                    )?
                                },
                                DbBlockType::Parent(parent) => {
                                    self.prove_block_parent(
                                        parent.block_id,
                                        parent.old_block_number,
                                        parent.old_min,
                                        parent.old_max,
                                        parent.prev_left_child.to_owned(),
                                        parent.prev_right_child.to_owned(),
                                        parent.old_rows_tree_hash.to_owned(),
                                        parent.extraction_proof.to_owned(),
                                        parent.rows_proof.to_owned(),
                                    )?
                                },
                                DbBlockType::Membership(membership) => {
                                    self.prove_membership(
                                        membership.block_id,
                                        membership.index_value,
                                        membership.old_min,
                                        membership.old_max,
                                        membership.left_child.to_owned(),
                                        membership.rows_tree_hash.to_owned(),
                                        last_proof.take().unwrap(),
                                    )?
                                },
                            });
                        }
                        last_proof.take().unwrap()
                    },
                    DatabaseType::IVC(ivc) => {
                        self.prove_ivc(
                            ivc.index_proof.to_owned(),
                            ivc.previous_ivc_proof.to_owned(),
                        )?
                    },
                }
            },
        })
    }
}
