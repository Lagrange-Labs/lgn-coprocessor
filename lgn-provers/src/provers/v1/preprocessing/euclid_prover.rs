use std::collections::HashMap;

use alloy::primitives::U256;
use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::v1::preprocessing::db_tasks::DatabaseType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbBlockType;
use lgn_messages::types::v1::preprocessing::db_tasks::DbRowType;
use lgn_messages::types::v1::preprocessing::ext_tasks::ExtractionType;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtraction;
use lgn_messages::types::v1::preprocessing::ext_tasks::FinalExtractionType;
use lgn_messages::types::v1::preprocessing::node_type;
use lgn_messages::types::v1::preprocessing::ConcreteValueExtractionCircuitInput;
use lgn_messages::types::v1::preprocessing::NodeType;
use lgn_messages::types::v1::preprocessing::WorkerTask;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::v1::ConcreteCircuitInput;
use lgn_messages::types::v1::ConcretePublicParameters;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::TaskType;
use mp2_common::poseidon::empty_poseidon_hash_as_vec;
use mp2_common::types::HashOutput;
use mp2_v1::api::generate_proof;
use mp2_v1::api::CircuitInput;
use mp2_v1::block_extraction;
use mp2_v1::contract_extraction;
use mp2_v1::final_extraction;
use mp2_v1::length_extraction;
use mp2_v1::length_extraction::LengthCircuitInput;
use mp2_v1::values_extraction;
use tracing::debug;

use crate::params;
use crate::provers::LgnProver;

pub struct EuclidProver {
    params: ConcretePublicParameters,
}

impl EuclidProver {
    pub fn new(params: ConcretePublicParameters) -> Self {
        Self { params }
    }

    pub fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksums: &HashMap<String, blake3::Hash>,
    ) -> anyhow::Result<Self> {
        let params = params::prepare_raw(url, dir, file, checksums)?;
        let reader = std::io::BufReader::new(params.as_ref());
        let params = bincode::deserialize_from(reader)?;
        Ok(Self { params })
    }

    fn prove(
        &self,
        input: ConcreteCircuitInput,
        name: &str,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("Proving {}", name);

        let now = std::time::Instant::now();

        match generate_proof(&self.params, input) {
            Ok(proof) => {
                debug!(
                    time = now.elapsed().as_secs_f32(),
                    proof_type = name,
                    "proof generation time: {:?}",
                    now.elapsed()
                );
                debug!("{name} size in kB: {}", proof.len() / 1024);
                Ok(proof)
            },
            Err(err) => {
                debug!("Proof generation failed in {:?}", now.elapsed());
                Err(err)
            },
        }
    }
}

impl EuclidProver {
    fn prove_value_extraction(
        &self,
        circuit_input: ConcreteValueExtractionCircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove(
            CircuitInput::ValuesExtraction(circuit_input),
            "value extraction",
        )
    }

    fn prove_length_extraction(
        &self,
        circuit_input: length_extraction::LengthCircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove(
            CircuitInput::LengthExtraction(circuit_input),
            "length extraction",
        )
    }

    fn prove_contract_extraction(
        &self,
        circuit_input: contract_extraction::CircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove(
            CircuitInput::ContractExtraction(circuit_input),
            "contract extraction",
        )
    }

    fn prove_block(
        &self,
        rlp_header: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = CircuitInput::BlockExtraction(
            block_extraction::CircuitInput::from_block_header(rlp_header),
        );
        self.prove(input, "block")
    }

    fn prove_final_extraction(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        value_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input =
            CircuitInput::FinalExtraction(final_extraction::CircuitInput::new_simple_input(
                block_proof,
                contract_proof,
                value_proof,
            )?);
        self.prove(input, "final extraction simple")
    }

    fn prove_final_extraction_lengthed(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        value_proof: Vec<u8>,
        length_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input =
            CircuitInput::FinalExtraction(final_extraction::CircuitInput::new_lengthed_input(
                block_proof,
                contract_proof,
                value_proof,
                length_proof,
            )?);
        self.prove(input, "final extraction lengthed")
    }

    fn prove_final_extraction_merge(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        simple_table_proof: Vec<u8>,
        mapping_table_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = CircuitInput::FinalExtraction(
            final_extraction::CircuitInput::new_merge_single_and_mapping(
                block_proof,
                contract_proof,
                simple_table_proof,
                mapping_table_proof,
            )?,
        );
        self.prove(input, "final extraction merge")
    }

    fn prove_cells_tree(
        &self,
        circuit_input: verifiable_db::cells_tree::CircuitInput,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove(CircuitInput::CellsTree(circuit_input), "cells tree")
    }

    fn prove_row_leaf(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let cells_proof = if !cells_proof.is_empty() {
            cells_proof
        } else {
            self.params.empty_cell_tree_proof()?
        };

        let input = CircuitInput::RowsTree(verifiable_db::row_tree::CircuitInput::leaf(
            identifier,
            value,
            is_multiplier,
            todo!(),
            cells_proof,
        )?);
        self.prove(input, "row leaf")
    }

    #[allow(clippy::too_many_arguments)]
    fn prove_row_partial(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        is_child_left: bool,
        child_proof: Vec<u8>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let cells_proof = if !cells_proof.is_empty() {
            cells_proof
        } else {
            self.params.empty_cell_tree_proof()?
        };
        let input = CircuitInput::RowsTree(verifiable_db::row_tree::CircuitInput::partial(
            identifier,
            value,
            is_multiplier,
            is_child_left,
            todo!(),
            child_proof,
            cells_proof,
        )?);
        self.prove(input, "row partial")
    }

    fn prove_row_full(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proofs: Vec<Vec<u8>>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let cells_proof = if !cells_proof.is_empty() {
            cells_proof
        } else {
            self.params.empty_cell_tree_proof()?
        };
        let input = CircuitInput::RowsTree(verifiable_db::row_tree::CircuitInput::full(
            identifier,
            value,
            is_multiplier,
            todo!(),
            child_proofs[0].to_owned(),
            child_proofs[1].to_owned(),
            cells_proof,
        )?);
        self.prove(input, "row full")
    }

    fn prove_block_leaf(
        &self,
        block_id: u64,
        extraction_proof: Vec<u8>,
        rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let block_id: u64 = u64::from_be_bytes(block_id.to_be_bytes());
        let input = CircuitInput::BlockTree(verifiable_db::block_tree::CircuitInput::new_leaf(
            block_id,
            extraction_proof,
            rows_tree_proof,
        ));
        self.prove(input, "block tree leaf")
    }

    #[allow(clippy::too_many_arguments)]
    fn prove_block_parent(
        &self,
        block_id: u64,
        old_block_number: U256,
        old_min: U256,
        old_max: U256,
        left_child: Option<HashOutput>,
        right_child: Option<HashOutput>,
        old_rows_tree_hash: HashOutput,
        extraction_proof: Vec<u8>,
        rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let left_hash =
            left_child.unwrap_or_else(|| empty_poseidon_hash_as_vec().try_into().unwrap());
        let right_hash =
            right_child.unwrap_or_else(|| empty_poseidon_hash_as_vec().try_into().unwrap());
        let input = CircuitInput::BlockTree(verifiable_db::block_tree::CircuitInput::new_parent(
            block_id,
            old_block_number,
            old_min,
            old_max,
            &left_hash,
            &right_hash,
            &(old_rows_tree_hash),
            extraction_proof,
            rows_tree_proof,
        ));
        self.prove(input, "block tree parent")
    }

    #[allow(clippy::too_many_arguments)]
    fn prove_membership(
        &self,
        block_id: u64,
        index_value: U256,
        old_min: U256,
        old_max: U256,
        left_child: HashOutput,
        rows_tree_hash: HashOutput,
        right_child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input =
            CircuitInput::BlockTree(verifiable_db::block_tree::CircuitInput::new_membership(
                block_id,
                index_value,
                old_min,
                old_max,
                &(left_child),
                &(rows_tree_hash),
                right_child_proof,
            ));
        self.prove(input, "membership")
    }

    fn prove_ivc(
        &self,
        index_proof: Vec<u8>,
        previous_proof: Option<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = match previous_proof {
            Some(previous_proof) => {
                CircuitInput::IVC(verifiable_db::ivc::CircuitInput::new_subsequent_input(
                    index_proof,
                    previous_proof,
                )?)
            },
            None => {
                CircuitInput::IVC(verifiable_db::ivc::CircuitInput::new_first_input(
                    index_proof,
                )?)
            },
        };

        self.prove(input, "ivc")
    }

    pub fn run_inner(
        &self,
        task: WorkerTask,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(match task.task_type {
            WorkerTaskType::Extraction(extraction) => {
                match extraction {
                    ExtractionType::MptExtraction(mpt) => {
                        self.prove_value_extraction(mpt.circuit_input)?
                    },
                    ExtractionType::LengthExtraction(length) => {
                        let mut nodes = length.nodes;

                        nodes.reverse();
                        let first = nodes.pop().context("Missing length extraction leaf node")?;

                        if node_type(&first)? != NodeType::Leaf {
                            bail!("The first node for a length extraction must be a leaf node");
                        }

                        let mut proof =
                            self.prove_length_extraction(LengthCircuitInput::new_leaf(
                                length.length_slot as u8,
                                first,
                                length.variable_slot as u8,
                            ))?;

                        for node in nodes {
                            match node_type(&node)? {
                                NodeType::Branch => {
                                    proof = self.prove_length_extraction(
                                        LengthCircuitInput::new_branch(node, proof),
                                    )?;
                                },
                                NodeType::Extension => {
                                    proof = self.prove_length_extraction(
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

                        let mut proof = self.prove_contract_extraction(
                            contract_extraction::CircuitInput::new_leaf(
                                first,
                                &contract.storage_root,
                                contract.contract,
                            ),
                        )?;

                        for node in nodes {
                            match node_type(&node)? {
                                NodeType::Branch => {
                                    proof = self.prove_contract_extraction(
                                        contract_extraction::CircuitInput::new_branch(node, proof),
                                    )?;
                                },
                                NodeType::Extension => {
                                    proof = self.prove_contract_extraction(
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
                        self.prove_block(block.rlp_header.to_owned())?
                    },
                    ExtractionType::FinalExtraction(final_extraction) => {
                        match *final_extraction {
                            FinalExtraction::Single(single_table_extraction) => {
                                match single_table_extraction.extraction_type {
                                    FinalExtractionType::Simple => {
                                        self.prove_final_extraction(
                                            single_table_extraction.block_proof.clone(),
                                            single_table_extraction.contract_proof.clone(),
                                            single_table_extraction.value_proof.clone(),
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
                    DatabaseType::Cell { circuit_input, .. } => {
                        self.prove_cells_tree(circuit_input)?
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

impl LgnProver for EuclidProver {
    fn run(
        &self,
        envelope: lgn_messages::types::MessageEnvelope,
    ) -> anyhow::Result<lgn_messages::types::MessageReplyEnvelope> {
        let task_id = envelope.task_id.clone();

        match envelope.task {
            TaskType::V1Preprocessing(task) => {
                let proof = self.run_inner(task)?;
                Ok(MessageReplyEnvelope::new(task_id, proof))
            },
            TaskType::V1Query(..) => {
                bail!(
                    "EuclidProver: unsupported task type. task_type: V1Query task_id: {}",
                    task_id,
                )
            },
            TaskType::V1Groth16(_revelation_proof) => {
                bail!(
                    "EuclidProver: unsupported task type. task_type: V1Groth16 task_id: {}",
                    task_id
                )
            },
        }
    }
}
