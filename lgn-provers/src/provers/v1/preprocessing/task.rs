use lgn_messages::types::v1::preprocessing::keys::ProofKey;
use lgn_messages::types::v1::preprocessing::task::{
    ExtractionType, FinalExtractionType, MptType, WorkerTask, WorkerTaskType,
};
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
        if let TaskType::V1Preprocessing(task @ WorkerTask { .. }) = envelope.inner() {
            let proof = self.run_inner(task)?;
            let key: ProofKey = envelope.inner().into();
            let reply_type = ReplyType::V1Preprocessing(WorkerReply::new(
                task.chain_id,
                Some((key.to_string(), proof)),
            ));
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

    fn run_inner(&mut self, task: &WorkerTask) -> anyhow::Result<Vec<u8>> {
        Ok(match &task.task_type {
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
            WorkerTaskType::Database(_db) => {
                todo!("Database prover")
            }
        })
    }
}
