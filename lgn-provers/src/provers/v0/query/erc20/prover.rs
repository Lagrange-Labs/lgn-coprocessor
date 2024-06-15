use crate::params::ParamsLoader;
use ethers::addressbook::Address;
use ethers::prelude::U256;
use mr_plonky2_circuits::api::{QueryInput, QueryParameters};
use lgn_messages::types::v0::query::erc20::{
    BlockFullNodeInput, BlockPartialNodeInput, RevelationData, StateInput, StorageBranchInput,
    StorageLeafInput,
};
use lgn_messages::types::Position;
use mr_plonky2_circuits::query_erc20;
use mr_plonky2_circuits::query_erc20::RevelationErcInput;
use mr_plonky2_circuits::types::HashOutput;
use tracing::{debug, info};

pub trait QueryProver {
    fn prove_storage_leaf(
        &self,
        contract: Address,
        data: &StorageLeafInput,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_storage_branch(&self, data: &StorageBranchInput) -> anyhow::Result<Vec<u8>>;

    fn prove_state_db(&self, contract: Address, data: &StateInput) -> anyhow::Result<Vec<u8>>;

    fn prove_block_partial_node(&self, data: &BlockPartialNodeInput) -> anyhow::Result<Vec<u8>>;

    fn prove_block_full_node(&self, data: &BlockFullNodeInput) -> anyhow::Result<Vec<u8>>;

    fn prove_revelation(&self, data: &RevelationData) -> anyhow::Result<Vec<u8>>;
}

pub struct EuclidProver {
    params: QueryParameters<21, 5>,
}

impl EuclidProver {
    pub fn init(url: &str, dir: &str, file: &str, skip_store: bool) -> anyhow::Result<Self> {
        info!("Creating Erc20 query prover");

        let params = ParamsLoader::prepare_bincode(url, dir, file, skip_store)
            .expect("Failed to load params");

        info!("Erc20 query prover created");

        Ok(Self { params })
    }
}

impl QueryProver for EuclidProver {
    fn prove_storage_leaf(
        &self,
        address: Address,
        data: &StorageLeafInput,
    ) -> anyhow::Result<Vec<u8>> {
        info!("Generating storage leaf proof...");

        let now = std::time::Instant::now();

        let circuit_input = query_erc20::StorageCircuitInput::new_leaf(
            address,
            data.query_address,
            data.value,
            data.total_supply,
            data.rewards_rate,
        );
        let input = query_erc20::CircuitInput::Storage(circuit_input);
        let input = QueryInput::QueryErc(input);

        let proof = self.params.generate_proof(input)?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "storage-leaf",
            "Storage entry leaf proof generation took: {:?}",
            now.elapsed()
        );
        debug!(
            "Storage entry leaf proof size in kb: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_storage_branch(&self, data: &StorageBranchInput) -> anyhow::Result<Vec<u8>> {
        info!("Generating storage branch proof...");

        let now = std::time::Instant::now();

        let circuit_input = query_erc20::StorageCircuitInput::new_inner_node(
            &data.left_child,
            &data.right_child,
            data.proved_is_right,
        );
        let circuit = query_erc20::CircuitInput::Storage(circuit_input);
        let input = QueryInput::QueryErc(circuit);

        let proof = self.params.generate_proof(input)?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "storage-branch",
            "Storage entry branch proof generation took: {:?}",
            now.elapsed()
        );
        debug!(
            "Storage entry branch proof size in kb: {}",
            proof.len() / 1024
        );

        Ok(proof)
    }

    fn prove_state_db(&self, contract: Address, data: &StateInput) -> anyhow::Result<Vec<u8>> {
        info!("Generating state db proof...");

        let now = std::time::Instant::now();

        let proof = data.proof.clone().unwrap_or(vec![]);
        let siblings = proof
            .clone()
            .into_iter()
            .map(|(_, hash)| hash)
            .collect::<Vec<[u8; 32]>>();

        let positions = proof
            .into_iter()
            .map(|(pos, _)| pos.index % 2 == 1)
            .collect::<Vec<bool>>();

        let depth = siblings.len() as u32;

        let circuit_input = query_erc20::StateCircuitInput::new(
            contract,
            data.mapping_slot,
            data.length_slot,
            data.block_number as u32,
            depth,
            &siblings,
            &positions,
            data.block_hash,
            data.storage_proof.clone(),
        )?;
        let circuit = query_erc20::CircuitInput::State(circuit_input);
        let input = QueryInput::QueryErc(circuit);

        let proof = self.params.generate_proof(input)?;

        debug!(
            time = now.elapsed().as_secs_f32(),
            proof_type = "state-db",
            "State db proof generation took: {:?}",
            now.elapsed()
        );
        debug!("State db proof size in kb: {}", proof.len() / 1024);

        Ok(proof)
    }

    fn prove_block_partial_node(&self, data: &BlockPartialNodeInput) -> anyhow::Result<Vec<u8>> {
        info!("Generating block partial node proof...");

        let now = std::time::Instant::now();

        let circuit_input = query_erc20::BlockCircuitInput::new_partial_node(
            data.child_proof.clone(),
            data.sibling_hash,
            data.sibling_is_left,
        )?;

        let circuit = query_erc20::CircuitInput::Block(circuit_input);
        let input = QueryInput::QueryErc(circuit);

        let proof = self.params.generate_proof(input)?;

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

    fn prove_block_full_node(&self, data: &BlockFullNodeInput) -> anyhow::Result<Vec<u8>> {
        info!("Generating block full node proof...");

        let now = std::time::Instant::now();

        let circuit_input = query_erc20::BlockCircuitInput::new_full_node(
            data.left_proof.clone(),
            data.right_proof.clone(),
        )?;
        let circuit = query_erc20::CircuitInput::Block(circuit_input);
        let input = QueryInput::QueryErc(circuit);

        let proof = self.params.generate_proof(input)?;

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
        info!("Generating revelation proof...");

        let now = std::time::Instant::now();

        let input = RevelationErcInput::new(
            data.query_min_block,
            data.query_max_block,
            data.erc2_proof.to_vec(),
            data.block_db_proof.to_vec(),
        )?;

        let input = query_erc20::CircuitInput::Revelation(input);
        let input = QueryInput::QueryErc(input);

        let proof = self.params.generate_proof(input)?;

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
