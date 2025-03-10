use std::collections::HashMap;

use alloy::primitives::U256;
use anyhow::bail;
use anyhow::Context;
use lgn_messages::types::v1::preprocessing::db_tasks::DbBlockType;
use lgn_messages::types::v1::preprocessing::node_type;
use lgn_messages::types::v1::preprocessing::NodeType;
use lgn_messages::types::v1::preprocessing::WorkerTaskType;
use lgn_messages::types::v1::ConcreteCircuitInput;
use lgn_messages::types::v1::ConcretePublicParameters;
use lgn_messages::types::MessageReplyEnvelope;
use lgn_messages::types::TaskType;
use lgn_messages::Proof;
use mp2_common::poseidon::empty_poseidon_hash_as_vec;
use mp2_common::types::HashOutput;
use mp2_v1::api::generate_proof;
use mp2_v1::api::CircuitInput;
use mp2_v1::contract_extraction;
use mp2_v1::length_extraction;
use mp2_v1::length_extraction::LengthCircuitInput;
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
    ) -> anyhow::Result<Proof> {
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
    fn prove_length_extraction(
        &self,
        circuit_input: length_extraction::LengthCircuitInput,
    ) -> anyhow::Result<Proof> {
        self.prove(
            CircuitInput::LengthExtraction(circuit_input),
            "length extraction",
        )
    }

    fn prove_contract_extraction(
        &self,
        circuit_input: contract_extraction::CircuitInput,
    ) -> anyhow::Result<Proof> {
        self.prove(
            CircuitInput::ContractExtraction(circuit_input),
            "contract extraction",
        )
    }

    fn prove_block_leaf(
        &self,
        block_id: u64,
        extraction_proof: Proof,
        rows_tree_proof: Proof,
    ) -> anyhow::Result<Proof> {
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
        extraction_proof: Proof,
        rows_tree_proof: Proof,
    ) -> anyhow::Result<Proof> {
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
        right_child_proof: Proof,
    ) -> anyhow::Result<Proof> {
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

    pub fn run_inner(
        &self,
        task: WorkerTaskType,
    ) -> anyhow::Result<Proof> {
        Ok(match task {
            WorkerTaskType::CircuitInput(circuit_input) => {
                self.prove(circuit_input, "circuit_input")?
            },
            WorkerTaskType::BatchedIndex(batched_index) => {
                let mut last_proof = None;
                for input in &batched_index.inputs {
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
            WorkerTaskType::BatchedLength(batched_length) => {
                let mut nodes = batched_length.nodes;

                nodes.reverse();
                let first = nodes.pop().context("Missing length extraction leaf node")?;

                if node_type(&first)? != NodeType::Leaf {
                    bail!("The first node for a length extraction must be a leaf node");
                }

                let mut proof = self.prove_length_extraction(LengthCircuitInput::new_leaf(
                    batched_length.length_slot as u8,
                    first,
                    batched_length.variable_slot as u8,
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
            WorkerTaskType::BatchedContract(batched_contract) => {
                let mut nodes = batched_contract.nodes;

                nodes.reverse();
                let first = nodes
                    .pop()
                    .context("Missing contract extraction leaf node")?;

                if node_type(&first)? != NodeType::Leaf {
                    bail!("The first node for a contract extraction must be a leaf node");
                }

                let mut proof =
                    self.prove_contract_extraction(contract_extraction::CircuitInput::new_leaf(
                        first,
                        &batched_contract.storage_root,
                        batched_contract.contract,
                    ))?;

                for node in nodes {
                    match node_type(&node)? {
                        NodeType::Branch => {
                            proof = self.prove_contract_extraction(
                                contract_extraction::CircuitInput::new_branch(node, proof),
                            )?;
                        },
                        NodeType::Extension => {
                            proof = self.prove_contract_extraction(
                                contract_extraction::CircuitInput::new_extension(node, proof),
                            )?;
                        },
                        NodeType::Leaf => bail!("Only the first node can be a leaf"),
                    }
                }

                proof
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
