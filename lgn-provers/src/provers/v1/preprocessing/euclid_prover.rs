use std::collections::HashMap;

use alloy::primitives::Address;
use alloy::primitives::U256;
use anyhow::bail;
use ethers::utils::rlp::Prototype;
use ethers::utils::rlp::Rlp;
use mp2_common::digest::TableDimension;
use mp2_common::poseidon::empty_poseidon_hash_as_vec;
use mp2_common::types::HashOutput;
use mp2_v1::api::generate_proof;
use mp2_v1::api::CircuitInput::BlockExtraction;
use mp2_v1::api::CircuitInput::BlockTree;
use mp2_v1::api::CircuitInput::CellsTree;
use mp2_v1::api::CircuitInput::ContractExtraction;
use mp2_v1::api::CircuitInput::FinalExtraction;
use mp2_v1::api::CircuitInput::LengthExtraction;
use mp2_v1::api::CircuitInput::RowsTree;
use mp2_v1::api::CircuitInput::ValuesExtraction;
use mp2_v1::api::CircuitInput::IVC;
use mp2_v1::api::CircuitInput::{
    self,
};
use mp2_v1::api::PublicParameters;
use mp2_v1::block_extraction;
use mp2_v1::contract_extraction;
use mp2_v1::final_extraction;
use mp2_v1::length_extraction::LengthCircuitInput;
use mp2_v1::values_extraction;
use tracing::debug;

use crate::params;
use crate::provers::v1::preprocessing::prover::StorageDatabaseProver;
use crate::provers::v1::preprocessing::prover::StorageExtractionProver;

pub struct EuclidProver {
    params: PublicParameters,
}

impl EuclidProver {
    pub fn new(params: PublicParameters) -> Self {
        Self { params }
    }

    pub(crate) fn init(
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
        input: CircuitInput,
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

impl StorageExtractionProver for EuclidProver {
    fn prove_single_variable_leaf(
        &self,
        node: Vec<u8>,
        slot: u8,
        column_id: u64,
    ) -> anyhow::Result<Vec<u8>> {
        let input = ValuesExtraction(values_extraction::CircuitInput::new_single_variable_leaf(
            node, slot, column_id,
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
        slot: u8,
        key_id: u64,
        value_id: u64,
    ) -> anyhow::Result<Vec<u8>> {
        let input = ValuesExtraction(values_extraction::CircuitInput::new_mapping_variable_leaf(
            node, slot, key, key_id, value_id,
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
            },
            Prototype::List(17) => {
                let input = ValuesExtraction(
                    values_extraction::CircuitInput::new_mapping_variable_branch(
                        node,
                        child_proofs,
                    ),
                );
                self.prove(input, "mapping variable branch")
            },
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

    fn prove_length_branch(
        &self,
        node: Vec<u8>,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
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

    fn prove_block(
        &self,
        rlp_header: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
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
        dimension: TableDimension,
    ) -> anyhow::Result<Vec<u8>> {
        let input = FinalExtraction(final_extraction::CircuitInput::new_simple_input(
            block_proof,
            contract_proof,
            value_proof,
            dimension,
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

    fn prove_final_extraction_merge(
        &self,
        block_proof: Vec<u8>,
        contract_proof: Vec<u8>,
        simple_table_proof: Vec<u8>,
        mapping_table_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = FinalExtraction(
            final_extraction::CircuitInput::new_merge_single_and_mapping(
                block_proof,
                contract_proof,
                simple_table_proof,
                mapping_table_proof,
            )?,
        );
        self.prove(input, "final extraction merge")
    }
}

impl StorageDatabaseProver for EuclidProver {
    fn prove_cell_leaf(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
    ) -> anyhow::Result<Vec<u8>> {
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::leaf(
            identifier,
            value,
            is_multiplier,
        ));
        self.prove(input, "cell leaf")
    }

    fn prove_cell_partial(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::partial(
            identifier,
            value,
            is_multiplier,
            child_proof,
        ));
        self.prove(input, "cell partial")
    }

    fn prove_cell_full(
        &self,
        identifier: u64,
        value: U256,
        is_multiplier: bool,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        let child_proofs = [child_proofs[0].to_owned(), child_proofs[1].to_vec()];
        let input = CellsTree(verifiable_db::cells_tree::CircuitInput::full(
            identifier,
            value,
            is_multiplier,
            child_proofs,
        ));
        self.prove(input, "cell full")
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

        let input = RowsTree(verifiable_db::row_tree::CircuitInput::leaf(
            identifier,
            value,
            is_multiplier,
            cells_proof,
        )?);
        self.prove(input, "row leaf")
    }

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
        let input = RowsTree(verifiable_db::row_tree::CircuitInput::partial(
            identifier,
            value,
            is_multiplier,
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
        is_multiplier: bool,
        child_proofs: Vec<Vec<u8>>,
        cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        let cells_proof = if !cells_proof.is_empty() {
            cells_proof
        } else {
            self.params.empty_cell_tree_proof()?
        };
        let input = RowsTree(verifiable_db::row_tree::CircuitInput::full(
            identifier,
            value,
            is_multiplier,
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
        let input = BlockTree(verifiable_db::block_tree::CircuitInput::new_parent(
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
                IVC(verifiable_db::ivc::CircuitInput::new_subsequent_input(
                    index_proof,
                    previous_proof,
                )?)
            },
            None => {
                IVC(verifiable_db::ivc::CircuitInput::new_first_input(
                    index_proof,
                )?)
            },
        };

        self.prove(input, "ivc")
    }
}
