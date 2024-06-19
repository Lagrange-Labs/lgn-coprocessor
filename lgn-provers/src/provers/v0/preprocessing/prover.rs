use anyhow::bail;
use tracing::debug;

use crate::params::ParamsLoader;
use ethers::utils::rlp::{Prototype, Rlp};
use lgn_messages::types::v0::preprocessing::{
    BlockLinkingInput, BlocksDbData, LengthExtractInput, MptProofBranchData, MptProofLeafData,
    StorageDbLeafData,
};
use mr_plonky2_circuits::api::{lpn_state, PublicParameters};
use mr_plonky2_circuits::{api, block, state, storage};

use crate::provers::v0::STORAGE_BLOCKCHAIN_DB_HEIGHT;

pub trait StorageProver {
    fn prove_mpt_leaf(&self, data: &MptProofLeafData) -> anyhow::Result<Vec<u8>>;
    fn prove_mpt_branch(&self, data: &MptProofBranchData) -> anyhow::Result<Vec<u8>>;
    fn prove_storage_db_leaf(&self, data: StorageDbLeafData) -> anyhow::Result<Vec<u8>>;

    fn prove_storage_db_branch(
        &self,
        left_proof: Vec<u8>,
        right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_state_db_leaf(&self, block_linking_proof: Vec<u8>) -> anyhow::Result<Vec<u8>>;
    fn prove_state_db_branch(
        &self,
        left_proof: Vec<u8>,
        right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;
    fn prove_length_extract(&self, data: LengthExtractInput) -> anyhow::Result<Vec<u8>>;

    fn prove_length_match(
        &self,
        mapping_proof: &[u8],
        length_extract_proof: &[u8],
        skip_match: bool,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_equivalence(
        &self,
        storage_proof: Vec<u8>,
        length_match_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;
    fn prove_block_number_linking(&self, data: &BlockLinkingInput) -> anyhow::Result<Vec<u8>>;

    fn prove_blocks_db_first(&self, block_leaf_index: BlocksDbData) -> anyhow::Result<Vec<u8>>;
    fn prove_blocks_db_subsequent(&self, data: BlocksDbData) -> anyhow::Result<Vec<u8>>;
}

pub(crate) struct StoragePreprocessProver {
    params: PublicParameters<STORAGE_BLOCKCHAIN_DB_HEIGHT>,
}

impl StoragePreprocessProver {
    // #[allow(dead_code)] - clippy warning because of dummy-prover feature
    #[allow(dead_code)]
    pub(crate) fn init(url: &str, dir: &str, file: &str, skip_store: bool) -> anyhow::Result<Self> {
        debug!("Creating preprocessing prover");
        let params = ParamsLoader::prepare_bincode(url, dir, file, skip_store)?;
        debug!("Preprocessing prover created");
        Ok(Self { params })
    }
}

impl StorageProver for StoragePreprocessProver {
    fn prove_mpt_leaf(&self, data: &MptProofLeafData) -> anyhow::Result<Vec<u8>> {
        let leaf = api::mapping::CircuitInput::new_leaf(
            data.node.clone(),
            data.storage_slot as usize,
            data.mapping_key.clone(),
        );

        let now = std::time::Instant::now();
        let input = api::CircuitInput::Mapping(leaf);
        match api::generate_proof(&self.params, input) {
            Ok(proof) => {
                debug!("MPT leaf proof size in kB: {}", proof.len() / 1024);
                debug!(
                    time = now.elapsed().as_secs_f32(),
                    proof_type = "mpt-leaf",
                    "MPT leaf proof generation time: {:?}",
                    now.elapsed()
                );
                Ok(proof)
            }
            Err(e) => bail!("Failed to generate proof: {:?}", e),
        }
    }

    fn prove_mpt_branch(&self, data: &MptProofBranchData) -> anyhow::Result<Vec<u8>> {
        let rlp = Rlp::new(&data.node);
        let child_proofs = &data.child_proofs;
        let input = match rlp.prototype()? {
            Prototype::List(2) => {
                debug!("proving mpt extension node: {:?}", data.hash);
                // Extension node has only 1 child
                assert_eq!(child_proofs.len(), 1);
                let child_proof = child_proofs[0].clone();
                api::mapping::CircuitInput::new_extension(data.node.clone(), child_proof)
            }
            Prototype::List(17) => {
                debug!("proving mpt branch node: {:?}", data.hash);
                api::mapping::CircuitInput::new_branch(data.node.clone(), child_proofs.to_vec())
            }
            _ => bail!("Invalid RLP item count"),
        };

        let input = api::CircuitInput::Mapping(input);

        let now = std::time::Instant::now();

        match api::generate_proof(&self.params, input) {
            Ok(proof) => {
                debug!("MPT branch proof size in kB: {}", proof.len() / 1024);
                debug!(
                    time = now.elapsed().as_secs_f32(),
                    proof_type = "mpt-branch",
                    "MPT branch proof generation time: {:?}",
                    now.elapsed()
                );
                Ok(proof)
            }
            Err(e) => {
                bail!("Failed to generate proof: {:?}", e);
            }
        }
    }

    fn prove_storage_db_leaf(&self, data: StorageDbLeafData) -> anyhow::Result<Vec<u8>> {
        let key = left_pad32(&data.key);
        let value = left_pad32(&data.value);

        let leaf_circuit_input = storage::lpn::LeafCircuit {
            mapping_key: key,
            mapping_value: value,
        };

        let leaf_input = storage::lpn::Input::Leaf(leaf_circuit_input);
        let storage_input = api::CircuitInput::Storage(leaf_input);

        let ts = std::time::Instant::now();
        let proof = api::generate_proof(&self.params, storage_input)
            .or_else(|e| bail!("Failed to generate storage leaf proof: {:?}", e))?;

        debug!("Storage leaf proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "storage-leaf",
            "Storage leaf proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_storage_db_branch(
        &self,
        left_proof: Vec<u8>,
        right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let ts = std::time::Instant::now();

        let node_circuit_input = storage::lpn::NodeInputs::new(left_proof, right_proof);

        let node_input = storage::lpn::Input::Node(node_circuit_input);
        let storage_input: api::CircuitInput<21> = api::CircuitInput::Storage(node_input);

        let proof = api::generate_proof(&self.params, storage_input)
            .or_else(|e| bail!("Failed to generate storage node proof: {:?}", e))?;

        debug!("Storage node proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "storage-node",
            "Storage node proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_state_db_leaf(&self, block_linking_proof: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING STATE LEAF");
        let ts = std::time::Instant::now();

        let leaf_circuit_input = lpn_state::api::CircuitInput::new_leaf(block_linking_proof);
        let leaf_circuit_input = api::CircuitInput::State(leaf_circuit_input);

        let proof = api::generate_proof(&self.params, leaf_circuit_input)
            .or_else(|e| bail!("Failed to generate state leaf proof: {:?}", e))?;

        debug!("State leaf proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "state-leaf",
            "State leaf proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_state_db_branch(
        &self,
        left_proof: Vec<u8>,
        right_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING STATE BRANCH");
        let ts = std::time::Instant::now();

        let input = lpn_state::api::CircuitInput::new_node(left_proof, right_proof);
        let input = api::CircuitInput::State(input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate state branch proof: {:?}", e))?;

        debug!("State branch proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "state-branch",
            "State branch proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_length_extract(&self, data: LengthExtractInput) -> anyhow::Result<Vec<u8>> {
        let ts = std::time::Instant::now();

        let length_extract_input = storage::length_extract::CircuitInput::new(
            data.length_slot,
            data.contract,
            data.mpt_nodes,
        );

        let input = api::CircuitInput::LengthExtract(length_extract_input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate length extract proof: {:?}", e))?;

        debug!("Length extract proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "length-extract",
            "Length extract proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_length_match(
        &self,
        mapping_proof: &[u8],
        length_extract_proof: &[u8],
        skip_match: bool,
    ) -> anyhow::Result<Vec<u8>> {
        let ts = std::time::Instant::now();

        let length_match_input = storage::length_match::CircuitInput::new(
            mapping_proof.to_vec(),
            length_extract_proof.to_vec(),
            skip_match,
        );

        let input = api::CircuitInput::LengthMatch(length_match_input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate length match proof: {:?}", e))?;

        debug!("Length match proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "length-match",
            "Length match proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_equivalence(
        &self,
        storage_proof: Vec<u8>,
        length_match_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING EQUIVALENCE");

        let ts = std::time::Instant::now();

        let digest_equal_input =
            storage::digest_equal::CircuitInput::new(storage_proof, length_match_proof);

        let input = api::CircuitInput::DigestEqual(digest_equal_input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate digest equal proof: {:?}", e))?;

        debug!("Digest equal proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "equivalence",
            "Digest equal proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_block_number_linking(&self, data: &BlockLinkingInput) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING BLOCK NUMBER LINKING");
        let ts = std::time::Instant::now();

        let address = data.contract;
        let block_linking_input = state::block_linking::CircuitInput::new(
            data.equivalence_proof.to_vec(),
            data.header_rlp.clone(),
            data.account_proof.clone(),
            address,
        );

        let input = api::CircuitInput::BlockLinking(block_linking_input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate block linking proof: {:?}", e))?;

        debug!("Block linking proof size in kB: {}", proof.len() / 1024);
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "block-linking",
            "Block linking proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }
    fn prove_blocks_db_first(&self, data: BlocksDbData) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING FIRST BLOCK DATABASE");
        let ts = std::time::Instant::now();

        let root_hash = *data.merkle_path.last().unwrap();
        // remove last element from merkle path
        let without_root = data.merkle_path[..data.merkle_path.len() - 1].to_vec();

        let block_tree_input =
            block::BlockTreeCircuit::new(data.leaf_index, root_hash, without_root);

        let block_input = block::CircuitInput::<21>::input_for_first_block(
            block_tree_input,
            data.new_leaf_proof.to_vec(),
        );

        let input = api::CircuitInput::BlockDB(block_input);

        let proof = api::generate_proof(&self.params, input)
            .or_else(|e| bail!("Failed to generate first block database proof: {:?}", e))?;

        debug!(
            "First block database proof size in kB: {}",
            proof.len() / 1024
        );
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "blocksdb-first",
            "First block database proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }

    fn prove_blocks_db_subsequent(&self, data: BlocksDbData) -> anyhow::Result<Vec<u8>> {
        debug!("PROVING SUBSEQUENT BLOCK DATABASE");
        let ts = std::time::Instant::now();

        let root_hash = *data.merkle_path.last().unwrap();

        // remove last element from merkle path
        let without_root = data.merkle_path[..data.merkle_path.len() - 1].to_vec();

        let block_tree_input =
            block::BlockTreeCircuit::new(data.leaf_index, root_hash, without_root);

        let block_input = block::CircuitInput::<21>::input_for_new_block(
            block_tree_input,
            data.new_leaf_proof.to_vec(),
            data.previous_leaf_proof.to_vec(),
        );

        let input = api::CircuitInput::BlockDB(block_input);

        let proof = api::generate_proof(&self.params, input).or_else(|e| {
            bail!(
                "Failed to generate subsequent block database proof: {:?}",
                e
            )
        })?;

        debug!(
            "Subsequent block database proof size in kB: {}",
            proof.len() / 1024
        );
        debug!(
            time = ts.elapsed().as_secs_f32(),
            proof_type = "blocksdb-subsequent",
            "Subsequent block database proof generation time: {:?}",
            ts.elapsed()
        );

        Ok(proof)
    }
}

fn left_pad32(slice: &[u8]) -> [u8; 32] {
    match slice.len() {
        a if a > 32 => panic!("left_pad32 must not be called with higher slice len than 32"),
        32 => slice.try_into().unwrap(),
        a => {
            let mut output = [0u8; 32];
            output[32 - a..].copy_from_slice(slice);
            output
        }
    }
}
