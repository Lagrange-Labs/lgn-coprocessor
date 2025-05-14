use anyhow::bail;
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
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::RequestVersioned;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;
use mp2_v1::api::TableRow;

use super::euclid_prover::PreprocessingEuclidProver;
use crate::provers::LgnProver;

impl LgnProver for PreprocessingEuclidProver {
    fn run(
        &self,
        envelope: RequestVersioned,
    ) -> anyhow::Result<MessageReplyEnvelope> {
        let query_id = envelope.query_id().to_owned();
        let task_id = envelope.task_id().to_owned();

        if let TaskType::V1Preprocessing(task @ WorkerTask { chain_id, .. }) = envelope.into_inner()
        {
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
            Ok(MessageReplyEnvelope::new(
                query_id.to_owned(),
                task_id.to_owned(),
                reply_type,
            ))
        } else {
            bail!("Unexpected task. task_id: {}", task_id);
        }
    }
}

impl PreprocessingEuclidProver {
    pub fn run_inner(
        &self,
        task: WorkerTask,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(match task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt) => {
                        match mpt.mpt_type {
                            MptType::VariableLeaf(variable_leaf) => {
                                self.prove_single_variable_leaf(
                                    variable_leaf.node,
                                    variable_leaf.slot,
                                    variable_leaf.evm_word,
                                    variable_leaf.table_info,
                                )?
                            },
                            MptType::MappingLeaf(mapping_leaf) => {
                                self.prove_mapping_variable_leaf(
                                    mapping_leaf.key,
                                    mapping_leaf.node,
                                    mapping_leaf.slot,
                                    mapping_leaf.key_id,
                                    mapping_leaf.evm_word,
                                    mapping_leaf.table_info,
                                )?
                            },
                            MptType::MappingBranch(mapping_branch) => {
                                self.prove_mapping_variable_branch(
                                    mapping_branch.node,
                                    mapping_branch.children_proofs.to_owned(),
                                )?
                            },
                            MptType::VariableBranch(variable_branch) => {
                                self.prove_single_variable_branch(
                                    variable_branch.node,
                                    variable_branch.children_proofs,
                                )?
                            },
                        }
                    },
                    ExtractionType::LengthExtraction(length) => {
                        let mut proofs = vec![];
                        for (i, node) in length.nodes.into_iter().enumerate() {
                            if i == 0 {
                                let proof = self.prove_length_leaf(
                                    node,
                                    length.length_slot,
                                    length.variable_slot,
                                )?;
                                proofs.push(proof);
                            } else {
                                self.prove_length_branch(node, proofs.last().unwrap().clone())?;
                            }
                        }
                        proofs.last().unwrap().clone()
                    },
                    ExtractionType::ContractExtraction(contract) => {
                        let mut proofs = vec![];
                        for (i, node) in contract.nodes.into_iter().enumerate() {
                            if i == 0 {
                                let proof = self.prove_contract_leaf(
                                    node,
                                    contract.storage_root.clone(),
                                    contract.contract,
                                )?;
                                proofs.push(proof);
                            } else {
                                let proof = self
                                    .prove_contract_branch(node, proofs.last().unwrap().clone())?;
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
                                    FinalExtractionType::Simple => {
                                        self.prove_final_extraction_simple(
                                            single_table_extraction.block_proof,
                                            single_table_extraction.contract_proof,
                                            single_table_extraction.value_proof,
                                        )?
                                    },
                                    FinalExtractionType::Lengthed => {
                                        self.prove_final_extraction_lengthed(
                                            single_table_extraction.block_proof,
                                            single_table_extraction.contract_proof,
                                            single_table_extraction.value_proof,
                                            single_table_extraction.length_proof,
                                        )?
                                    },
                                }
                            },
                            FinalExtraction::Merge(mapping_table_extraction) => {
                                self.prove_final_extraction_merge(
                                    mapping_table_extraction.block_proof,
                                    mapping_table_extraction.contract_proof,
                                    mapping_table_extraction.simple_table_proof,
                                    mapping_table_extraction.mapping_table_proof,
                                )?
                            },
                            FinalExtraction::Offchain(offchain_extraction) => {
                                let table_rows: Vec<_> = offchain_extraction
                                    .table_rows
                                    .into_iter()
                                    .map(TableRow::from)
                                    .collect();
                                self.prove_offchain_extraction_merge(
                                    offchain_extraction.primary_index,
                                    offchain_extraction.root_of_trust,
                                    offchain_extraction.prev_epoch_proof,
                                    &table_rows,
                                    &offchain_extraction.row_unique_columns,
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
                                    leaf.row_unique_data,
                                    leaf.cells_proof,
                                )?
                            },
                            DbRowType::Partial(partial) => {
                                self.prove_row_partial(
                                    partial.identifier,
                                    partial.value,
                                    partial.is_multiplier,
                                    partial.is_child_left,
                                    partial.row_unique_data,
                                    partial.child_proof.to_owned(),
                                    partial.cells_proof.to_owned(),
                                )?
                            },
                            DbRowType::Full(full) => {
                                self.prove_row_full(
                                    full.identifier,
                                    full.value,
                                    full.is_multiplier,
                                    full.row_unique_data,
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
                            ivc.provable_data_commitment,
                            ivc.index_proof.to_owned(),
                            ivc.previous_ivc_proof.to_owned(),
                        )?
                    },
                }
            },
        })
    }
}
