use anyhow::bail;
use anyhow::Context;
use ethers::utils::rlp::Rlp;
use lgn_messages::types::v1::preprocessing::db_keys;
use lgn_messages::types::v1::preprocessing::db_tasks::DatabaseType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbBlockType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbCellType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbRowType;
use lgn_messages::types::v1::preprocessing::ext_keys;
use lgn_messages::types::v1::preprocessing::ext_tasks::ExtractionType;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtraction;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtractionType;
use lgn_messages::types::v1::preprocessing::WorkerTask;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::MessageEnvelope;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::ProofCategory;
use lgn_messages::types::ReplyType;
use lgn_messages::types::TaskType;
use lgn_messages::types::WorkerReply;
use mp2_v1::contract_extraction;
use mp2_v1::length_extraction::LengthCircuitInput;

use crate::provers::v1::preprocessing::prover::PreprocessingProver;
use crate::provers::LgnProver;

/// Different types of node types.
#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Branch,
    Extension,
    Leaf,
}

/// Returns the node type given an encoded node.
///
/// The node spec is at [1].
///
/// 1- https://github.com/ethereum/execution-specs/blob/78fb726158c69d8fa164e28f195fabf6ab59b915/src/ethereum/cancun/trie.py#L177-L191
pub fn node_type(rlp_data: &[u8]) -> anyhow::Result<NodeType> {
    let rlp = Rlp::new(rlp_data);

    let item_count = rlp.item_count()?;

    if item_count == 17 {
        Ok(NodeType::Branch)
    } else if item_count == 2 {
        // The first item is the encoded path, if it begins with a 2 or 3 it is a leaf, else it is
        // an extension node
        let first_item = rlp.at(0)?;

        // We want the first byte
        let first_byte = first_item.as_raw()[0];

        // The we divide by 16 to get the first nibble
        match first_byte / 16 {
            0 | 1 => Ok(NodeType::Extension),
            2 | 3 => Ok(NodeType::Leaf),
            _ => {
                bail!("Expected compact encoding beginning with 0,1,2 or 3")
            },
        }
    } else {
        bail!("RLP encoded Node item count was {item_count}, expected either 17 or 2")
    }
}

pub struct Preprocessing<P> {
    prover: P,
}

impl<P: PreprocessingProver> LgnProver<TaskType, ReplyType> for Preprocessing<P> {
    fn run(
        &self,
        envelope: MessageEnvelope<TaskType>,
    ) -> anyhow::Result<MessageReplyEnvelope<ReplyType>> {
        let query_id = envelope.query_id.clone();
        let task_id = envelope.task_id.clone();

        match envelope.inner {
            TaskType::TxTrie(..) => {
                panic!("Unsupported task type. task_type: TxTrie")
            },
            TaskType::RecProof(..) => {
                panic!("Unsupported task type. task_type: RecProof")
            },
            TaskType::V1Preprocessing(task) => {
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
                    Some((key, result)),
                    ProofCategory::Querying,
                ));
                Ok(MessageReplyEnvelope::new(query_id, task_id, reply_type))
            },
            TaskType::V1Query(..) => {
                panic!("Unsupported task type. task_type: V1Query")
            },
            TaskType::V1Groth16(..) => panic!("Unsupported task type. task_type: V1Groth16"),
        }
    }
}

impl<P: PreprocessingProver> Preprocessing<P> {
    pub fn new(prover: P) -> Self {
        Self { prover }
    }

    pub fn run_inner(
        &self,
        task: WorkerTask,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(match task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt) => {
                        self.prover.prove_value_extraction(mpt.circuit_input)?
                    },
                    ExtractionType::LengthExtraction(length) => {
                        let mut nodes = length.nodes;

                        nodes.reverse();
                        let first = nodes.pop().context("Missing length extraction leaf node")?;

                        if node_type(&first)? != NodeType::Leaf {
                            bail!("The first node for a length extraction must be a leaf node");
                        }

                        let mut proof =
                            self.prover
                                .prove_length_extraction(LengthCircuitInput::new_leaf(
                                    length.length_slot as u8,
                                    first,
                                    length.variable_slot as u8,
                                ))?;

                        for node in nodes {
                            match node_type(&node)? {
                                NodeType::Branch => {
                                    proof = self.prover.prove_length_extraction(
                                        LengthCircuitInput::new_branch(node, proof),
                                    )?;
                                },
                                NodeType::Extension => {
                                    proof = self.prover.prove_length_extraction(
                                        LengthCircuitInput::new_extension(node, proof),
                                    )?;
                                },
                                NodeType::Leaf => bail!("Only the first node can be a leaf"),
                            }
                        }

                        proof
                    },
                    ExtractionType::ContractExtraction(contract) => {
                        let mut nodes = contract.nodes;

                        nodes.reverse();
                        let first = nodes
                            .pop()
                            .context("Missing contract extraction leaf node")?;

                        if node_type(&first)? != NodeType::Leaf {
                            bail!("The first node for a contract extraction must be a leaf node");
                        }

                        let mut proof = self.prover.prove_contract_extraction(
                            contract_extraction::CircuitInput::new_leaf(
                                first,
                                &contract.storage_root,
                                contract.contract,
                            ),
                        )?;

                        for node in nodes {
                            match node_type(&node)? {
                                NodeType::Branch => {
                                    proof = self.prover.prove_contract_extraction(
                                        contract_extraction::CircuitInput::new_branch(node, proof),
                                    )?;
                                },
                                NodeType::Extension => {
                                    proof = self.prover.prove_contract_extraction(
                                        contract_extraction::CircuitInput::new_extension(
                                            node, proof,
                                        ),
                                    )?;
                                },
                                NodeType::Leaf => bail!("Only the first node can be a leaf"),
                            }
                        }

                        proof
                    },
                    ExtractionType::BlockExtraction(block) => {
                        self.prover.prove_block(block.rlp_header.to_owned())?
                    },
                    ExtractionType::FinalExtraction(final_extraction) => {
                        match *final_extraction {
                            FinalExtraction::Single(single_table_extraction) => {
                                match single_table_extraction.extraction_type {
                                    FinalExtractionType::Simple(compound) => {
                                        self.prover.prove_final_extraction_simple(
                                            single_table_extraction.block_proof.clone(),
                                            single_table_extraction.contract_proof.clone(),
                                            single_table_extraction.value_proof.clone(),
                                            compound,
                                        )?
                                    },
                                    FinalExtractionType::Lengthed => {
                                        self.prover.prove_final_extraction_lengthed(
                                            single_table_extraction.block_proof.clone(),
                                            single_table_extraction.contract_proof.clone(),
                                            single_table_extraction.value_proof.clone(),
                                            single_table_extraction.length_proof.clone(),
                                        )?
                                    },
                                }
                            },
                            FinalExtraction::Merge(mapping_table_extraction) => {
                                self.prover.prove_final_extraction_merge(
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
                                self.prover.prove_cell_leaf(
                                    leaf.identifier,
                                    leaf.value,
                                    leaf.is_multiplier,
                                )?
                            },
                            DbCellType::Partial(branch) => {
                                self.prover.prove_cell_partial(
                                    branch.identifier,
                                    branch.value,
                                    branch.is_multiplier,
                                    branch.child_proof,
                                )?
                            },
                            DbCellType::Full(full) => {
                                self.prover.prove_cell_full(
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
                                self.prover.prove_row_leaf(
                                    leaf.identifier,
                                    leaf.value,
                                    leaf.is_multiplier,
                                    leaf.cells_proof,
                                )?
                            },
                            DbRowType::Partial(partial) => {
                                self.prover.prove_row_partial(
                                    partial.identifier,
                                    partial.value,
                                    partial.is_multiplier,
                                    partial.is_child_left,
                                    partial.child_proof.to_owned(),
                                    partial.cells_proof.to_owned(),
                                )?
                            },
                            DbRowType::Full(full) => {
                                self.prover.prove_row_full(
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
                                    self.prover.prove_block_leaf(
                                        leaf.block_id,
                                        leaf.extraction_proof.to_owned(),
                                        leaf.rows_proof.to_owned(),
                                    )?
                                },
                                DbBlockType::Parent(parent) => {
                                    self.prover.prove_block_parent(
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
                                    self.prover.prove_membership(
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
                        self.prover.prove_ivc(
                            ivc.index_proof.to_owned(),
                            ivc.previous_ivc_proof.to_owned(),
                        )?
                    },
                }
            },
        })
    }
}
