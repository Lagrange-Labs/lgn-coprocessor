use crate::params::ParamsLoader;
use crate::provers::v1::preprocessing::prover::{StorageDatabaseProver, StorageExtractionProver};
use alloy::primitives::{Address, U256};
use anyhow::bail;
use ethers::utils::rlp::{Prototype, Rlp};
use mp2_common::types::HashOutput;
use mp2_v1::api::CircuitInput::{
    BlockExtraction, BlockTree, CellsTree, ContractExtraction, FinalExtraction, LengthExtraction,
    RowsTree, ValuesExtraction,
};
use mp2_v1::api::{generate_proof, CircuitInput, PublicParameters};
use mp2_v1::length_extraction::LengthCircuitInput;
use mp2_v1::{block_extraction, contract_extraction, final_extraction, values_extraction};
use tracing::debug;

pub struct EuclidProver {
    params: PublicParameters,
}

impl EuclidProver {
    #[allow(dead_code)]
    pub(crate) fn new(params: PublicParameters) -> Self {
        Self { params }
    }

    #[allow(dead_code)]
    pub(crate) fn init(
        url: &str,
        dir: &str,
        file: &str,
        checksum_expected_local_path: &str,
        skip_checksum: bool,
        skip_store: bool,
    ) -> anyhow::Result<Self> {
        debug!("Creating preprocessing prover");
        let params = ParamsLoader::prepare_bincode(
            url,
            dir,
            file,
            checksum_expected_local_path,
            skip_checksum,
            skip_store,
        )?;
        debug!("Preprocessing prover created");
        Ok(Self { params })
    }

    fn prove(&self, input: CircuitInput, name: &str) -> anyhow::Result<Vec<u8>> {
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
            }
            Err(err) => {
                debug!("Proof generation failed in {:?}", now.elapsed());
                Err(err)
            }
        }
    }
}

impl StorageExtractionProver for EuclidProver {
    fn prove_single_variable_leaf(
        &self,
        node: Vec<u8>,
        slot: usize,
        contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        let alloy_address = &mut &contract_address.0.into();
        let input = ValuesExtraction(values_extraction::CircuitInput::new_single_variable_leaf(
            node,
            slot as u8,
            alloy_address,
        ));
        self.prove(input, "single variable leaf")
    }

    fn prove_single_variable_branch(
        &self,
        node: Vec<u8>,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove(
            ValuesExtraction(values_extraction::CircuitInput::new_single_variable_branch(
                node,
                child_proofs,
            )),
            "single variable branch",
        )
    }

    fn prove_mapping_variable_leaf(
        &self,
        key: Vec<u8>,
        node: Vec<u8>,
        slot: usize,
        contract_address: &Address,
    ) -> anyhow::Result<Vec<u8>> {
        let input = ValuesExtraction(values_extraction::CircuitInput::new_mapping_variable_leaf(
            node,
            slot as u8,
            key,
            contract_address,
        ));
        self.prove(input, "mapping variable leaf")
    }

    fn prove_mapping_variable_branch(
        &self,
        node: Vec<u8>,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        let rlp = Rlp::new(&node);
        match rlp.prototype()? {
            Prototype::List(2) => {
                let input = ValuesExtraction(values_extraction::CircuitInput::new_extension(
                    node,
                    child_proofs[0].to_owned(),
                ));
                self.prove(input, "mapping variable extension")
            }
            Prototype::List(17) => {
                let input = ValuesExtraction(
                    values_extraction::CircuitInput::new_mapping_variable_branch(
                        node,
                        child_proofs,
                    ),
                );
                self.prove(input, "mapping variable branch")
            }
            _ => bail!("Invalid RLP item count"),
        }
    }

    fn prove_length_leaf(
        &self,
        node: Vec<u8>,
        length_slot: usize,
        variable_slot: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let input = LengthExtraction(LengthCircuitInput::new_leaf(
            length_slot as u8,
            node,
            variable_slot as u8,
        ));
        self.prove(input, "length leaf")
    }

    fn prove_length_branch(&self, node: Vec<u8>, child_proof: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let input = LengthExtraction(LengthCircuitInput::new_branch(node, child_proof));
        self.prove(input, "length branch")
    }

    fn prove_contract_leaf(
        &self,
        node: Vec<u8>,
        storage_root: Vec<u8>,
        contract_address: Address,
    ) -> anyhow::Result<Vec<u8>> {
        let input = ContractExtraction(contract_extraction::CircuitInput::new_leaf(
            node,
            &storage_root,
            contract_address,
        ));
        self.prove(input, "contract leaf")
    }

    fn prove_contract_branch(
        &self,
        node: Vec<u8>,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = ContractExtraction(contract_extraction::CircuitInput::new_branch(
            node,
            child_proof,
        ));
        self.prove(input, "contract branch")
    }

    fn prove_block(&self, rlp_header: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let input = BlockExtraction(block_extraction::CircuitInput::from_block_header(
            rlp_header,
        ));
        self.prove(input, "block")
    }

    fn prove_final_extraction_simple(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        value_proof: Vec<u8>,
        compound: bool,
    ) -> anyhow::Result<Vec<u8>> {
        let input = FinalExtraction(final_extraction::CircuitInput::new_simple_input(
            block_proof,
            contract_proof,
            value_proof,
            compound,
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
        let input = FinalExtraction(final_extraction::CircuitInput::new_lengthed_input(
            block_proof,
            contract_proof,
            value_proof,
            length_proof,
        )?);
        self.prove(input, "final extraction lengthed")
    }
}

impl StorageDatabaseProver for EuclidProver {
    fn prove_cell_leaf(&self, identifier: u64, value: U256) -> anyhow::Result<Vec<u8>> {
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::leaf(
            identifier, value,
        ));
        self.prove(input, "cell leaf")
    }

    fn prove_cell_partial(
        &self,
        identifier: u64,
        value: U256,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::partial(
            identifier,
            value,
            child_proof,
        ));
        self.prove(input, "cell partial")
    }

    fn prove_cell_full(
        &self,
        identifier: u64,
        value: U256,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        let child_proofs = [child_proofs[0].to_owned(), child_proofs[1].to_vec()];
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::full(
            identifier,
            value,
            child_proofs,
        ));
        self.prove(input, "cell full")
    }

    fn prove_row_leaf(
        &self,
        identifier: u64,
        value: U256,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let cells_proof = if !cells_proof.is_empty() {
            cells_proof
        } else {
            // TODO: provide empty
            unimplemented!("No cells proof provided")
        };

        let input = RowsTree(verifiable_db::row_tree::CircuitInput::leaf(
            identifier,
            value,
            cells_proof,
        )?);
        self.prove(input, "row leaf")
    }

    fn prove_row_partial(
        &self,
        identifier: u64,
        value: U256,
        is_child_left: bool,
        child_proof: Vec<u8>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = RowsTree(verifiable_db::row_tree::CircuitInput::partial(
            identifier,
            value,
            is_child_left,
            child_proof,
            cells_proof,
        )?);
        self.prove(input, "row partial")
    }

    fn prove_row_full(
        &self,
        identifier: u64,
        value: U256,
        child_proofs: Vec<Vec<u8>>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = RowsTree(verifiable_db::row_tree::CircuitInput::full(
            identifier,
            value,
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
        let input = BlockTree(verifiable_db::block_tree::CircuitInput::new_leaf(
            block_id,
            extraction_proof,
            rows_tree_proof,
        ));
        self.prove(input, "block tree leaf")
    }

    fn prove_block_parent(
        &self,
        block_id: u64,
        old_block_number: U256,
        old_min: U256,
        old_max: U256,
        left_child: HashOutput,
        right_child: HashOutput,
        old_rows_tree_hash: HashOutput,
        extraction_proof: Vec<u8>,
        rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let block_id: u64 = u64::from_be_bytes(block_id.to_be_bytes());
        let input = BlockTree(verifiable_db::block_tree::CircuitInput::new_parent(
            block_id,
            old_block_number,
            old_min,
            old_max,
            &(left_child.into()),
            &(right_child.into()),
            &(old_rows_tree_hash.into()),
            extraction_proof,
            rows_tree_proof,
        ));
        self.prove(input, "block tree parent")
    }

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
        let input = BlockTree(verifiable_db::block_tree::CircuitInput::new_membership(
            block_id,
            index_value,
            old_min,
            old_max,
            &(left_child.into()),
            &(rows_tree_hash.into()),
            right_child_proof,
        ));
        self.prove(input, "membership")
    }
}
