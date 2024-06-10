use anyhow::bail;
use mr_plonky2_circuits::query2::PublicParameters as QueryParameters;

use mr_plonky2_circuits::query2::block::CircuitInput as BlockCircuitInput;
use mr_plonky2_circuits::query2::revelation::RevelationRecursiveInput as RevelationCircuitInputs;
use mr_plonky2_circuits::query2::state::CircuitInput as StateCircuitInput;

use lgn_messages::types::v0::query::{
    PartialNodeBlockData, Query2StateData, RevelationData, StorageProofInput,
};
use tracing::debug;

use crate::params::ParamsLoader;

use crate::provers::v0::EXPOSED_RESULT_SIZE;
use crate::provers::v0::STORAGE_BLOCKCHAIN_DB_HEIGHT;

pub trait QueryProver {
    fn prove_storage_entry(&self, data: StorageProofInput) -> anyhow::Result<Vec<u8>>;

    fn prove_state_db(&self, d: &Query2StateData) -> anyhow::Result<Vec<u8>>;

    fn prove_block_partial_node(&self, data: &PartialNodeBlockData) -> anyhow::Result<Vec<u8>>;

    fn prove_block_full_node(
        &self,
        left_proof: &[u8],
        right_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_revelation(&self, data: &RevelationData) -> anyhow::Result<Vec<u8>>;
}

pub struct QueryStorageProver {
    params: QueryParameters<STORAGE_BLOCKCHAIN_DB_HEIGHT, EXPOSED_RESULT_SIZE>,
}

impl QueryStorageProver {
    pub fn init(url: &str, dir: &str, file: &str, skip_store: bool) -> anyhow::Result<Self> {
        debug!("Creating QueryProver");
        let params = ParamsLoader::prepare_bincode(url, dir, file, skip_store)?;
        debug!("QueryProver created");
        Ok(Self { params })
    }
}

impl QueryProver for QueryStorageProver {
    fn prove_storage_entry(&self, data: StorageProofInput) -> anyhow::Result<Vec<u8>> {
        debug!("Generating storage proof...");

        let now = std::time::Instant::now();

        let storage_input = match data {
            StorageProofInput::Leaf { key, value } => {
                debug!("Generating proof for storage leaf with key: {:?}", key);
                mr_plonky2_circuits::query2::storage::CircuitInput::new_leaf(&key, &value)
            }
            StorageProofInput::FullBranch {
                left_child_proof,
                right_child_proof,
            } => {
                debug!("Generating proof for full branch");
                mr_plonky2_circuits::query2::storage::CircuitInput::new_full_node(
                    &left_child_proof,
                    &right_child_proof,
                )
            }
            StorageProofInput::PartialBranch {
                proven_child,
                unproven_child_hash,
                right_is_proven,
            } => {
                debug!("Generating proof for partial branch");
                if right_is_proven {
                    mr_plonky2_circuits::query2::storage::CircuitInput::new_partial_node(
                        &unproven_child_hash,
                        &proven_child,
                        right_is_proven,
                    )
                } else {
                    mr_plonky2_circuits::query2::storage::CircuitInput::new_partial_node(
                        &proven_child,
                        &unproven_child_hash,
                        right_is_proven,
                    )
                }
            }
        };

        let input = mr_plonky2_circuits::query2::api::CircuitInput::Storage(storage_input);

        let proof = self
            .params
            .generate_proof(input)
            .or_else(|e| bail!("Could not prove storage entry: {:?}", e))?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "storage-entry",
            "Storage entry proof generation took: {:?}",
            now.elapsed()
        );
        debug!("Storage entry proof size in kb: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_state_db(&self, d: &Query2StateData) -> anyhow::Result<Vec<u8>> {
        let now = std::time::Instant::now();

        let proof = d.proof.clone().unwrap_or_default();
        let siblings = proof
            .clone()
            .into_iter()
            .map(|(_, hash)| hash)
            .collect::<Vec<[u8; 32]>>();

        let positions = proof
            .into_iter()
            .map(|(pos, _)| pos.index % 2 == 0)
            .collect::<Vec<bool>>();

        let depth = siblings.len() as u32;

        let input = StateCircuitInput::new(
            d.smart_contract_address,
            d.mapping_slot,
            d.length_slot,
            // currently proving is assuming block number < 2^32
            d.block_number as u32,
            depth,
            &siblings,
            &positions,
            d.block_hash,
            // TODO: make this take a slice in mapreduce-plonky2
            d.storage_proof.to_vec(),
        )?;

        let input = mr_plonky2_circuits::query2::api::CircuitInput::State(input);

        let proof = self
            .params
            .generate_proof(input)
            .or_else(|e| bail!("Could not prove statedb: {:?}", e))?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "statedb",
            "State database proof generation took: {:?}",
            now.elapsed()
        );
        debug!("State database proof size in kb: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_block_partial_node(&self, data: &PartialNodeBlockData) -> anyhow::Result<Vec<u8>> {
        let now = std::time::Instant::now();

        let sibling_is_left = data.sibling_position.index % 2 == 0;
        debug!("Sibling is left: {:?}", sibling_is_left);

        let input = BlockCircuitInput::new_partial_node(
            data.child_proof.to_vec(),
            data.sibling_hash,
            sibling_is_left,
        )?;

        let input = mr_plonky2_circuits::query2::api::CircuitInput::Block(input);

        let proof = self
            .params
            .generate_proof(input)
            .or_else(|e| bail!("Could not prove block partial node: {:?}", e))?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "block-partial-node",
            "Block partial node proof generation took: {:?}",
            now.elapsed()
        );
        debug!(
            "Block partial node proof size in kb: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_block_full_node(
        &self,
        left_proof: &[u8],
        right_proof: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        let now = std::time::Instant::now();

        // TODO: make these slices in mapreduce-plonky2
        let input = BlockCircuitInput::new_full_node(left_proof.to_vec(), right_proof.to_vec())?;

        let input = mr_plonky2_circuits::query2::api::CircuitInput::Block(input);

        let proof = self
            .params
            .generate_proof(input)
            .or_else(|e| bail!("Could not prove block full node: {:?}", e))?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "block-full-node",
            "Block full node proof generation took: {:?}",
            now.elapsed()
        );
        debug!("Block full node proof size in kb: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_revelation(&self, data: &RevelationData) -> anyhow::Result<Vec<u8>> {
        let now = std::time::Instant::now();

        let input = RevelationCircuitInputs::<EXPOSED_RESULT_SIZE>::new(
            data.mapping_keys.clone(),
            data.query_min_block,
            data.query_max_block,
            // TODO: make these references in mapreduce-plonky2
            data.query2_proof.to_vec(),
            data.block_db_proof.to_vec(),
        )?;

        let input = mr_plonky2_circuits::query2::api::CircuitInput::Revelation(input);

        let proof = self
            .params
            .generate_proof(input)
            .or_else(|e| bail!("Could not prove revelation: {:?}", e))?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "revelation",
            "Revelation proof generation took: {:?}",
            now.elapsed()
        );
        debug!("Revelation proof size in kb: {}", proof.len() / 1024);

        Ok(proof)
    }
}
