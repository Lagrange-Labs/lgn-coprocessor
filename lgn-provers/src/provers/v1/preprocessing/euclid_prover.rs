use crate::params::ParamsLoader;
use crate::provers::v1::preprocessing::prover::{
    Hash, StorageDatabaseProver, StorageExtractionProver, F,
};
use anyhow::bail;
use ethers::addressbook::Address;
use ethers::prelude::U256;
use ethers::utils::rlp::{Prototype, Rlp};
use mp2_v1::api::CircuitInput::{
    BlockExtraction, ContractExtraction, FinalExtraction, LengthExtraction, ValuesExtraction,
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

    fn prove_extraction(&self, input: CircuitInput, name: &str) -> anyhow::Result<Vec<u8>> {
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
                debug!(
                    "Single variable leaf proof generation failed in {:?}",
                    now.elapsed()
                );
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
        self.prove_extraction(input, "single variable leaf")
    }

    fn prove_single_variable_branch(
        &self,
        node: Vec<u8>,
        child_proofs: Vec<Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        self.prove_extraction(
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
        let alloy_address = &mut &contract_address.0.into();
        let input = ValuesExtraction(values_extraction::CircuitInput::new_mapping_variable_leaf(
            node,
            slot as u8,
            key,
            alloy_address,
        ));
        self.prove_extraction(input, "mapping variable leaf")
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
                self.prove_extraction(input, "mapping variable extension")
            }
            Prototype::List(17) => {
                let input = ValuesExtraction(
                    values_extraction::CircuitInput::new_mapping_variable_branch(
                        node,
                        child_proofs,
                    ),
                );
                self.prove_extraction(input, "mapping variable branch")
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
        self.prove_extraction(input, "length leaf")
    }

    fn prove_length_branch(&self, node: Vec<u8>, child_proof: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let input = LengthExtraction(LengthCircuitInput::new_branch(node, child_proof));
        self.prove_extraction(input, "length branch")
    }

    fn prove_contract_leaf(
        &self,
        node: Vec<u8>,
        storage_root: Vec<u8>,
        contract_address: Address,
    ) -> anyhow::Result<Vec<u8>> {
        let alloy_address = &mut &contract_address.0.into();
        let input = ContractExtraction(contract_extraction::CircuitInput::new_leaf(
            node,
            &storage_root,
            **alloy_address,
        ));
        self.prove_extraction(input, "contract leaf")
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
        self.prove_extraction(input, "contract branch")
    }

    fn prove_block(&self, rlp_header: Vec<u8>) -> anyhow::Result<Vec<u8>> {
        let input = BlockExtraction(block_extraction::CircuitInput::from_block_header(
            rlp_header,
        ));
        self.prove_extraction(input, "block")
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
        self.prove_extraction(input, "final extraction simple")
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
        self.prove_extraction(input, "final extraction lengthed")
    }
}

impl StorageDatabaseProver for EuclidProver {
    fn prove_cell_leaf(&self, _identifier: F, _value: U256) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_cell_partial(
        &self,
        _identifier: F,
        _value: U256,
        _child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_cell_full(
        &self,
        _identifier: F,
        _value: U256,
        _child_proofs: [Vec<u8>; 2],
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_row_leaf(
        &self,
        _identifier: F,
        _value: U256,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_row_partial(
        &self,
        _identifier: F,
        _value: U256,
        _is_child_left: bool,
        _child_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_row_full(
        &self,
        _identifier: F,
        _value: U256,
        _left_proof: Vec<u8>,
        _right_proof: Vec<u8>,
        _cells_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_membership(
        _index_identifier: F,
        _index_value: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: Hash,
        _rows_tree_hash: Hash,
        _right_child_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_block_leaf(
        &self,
        _block_id: F,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    fn prove_block_parent(
        &self,
        _block_id: F,
        _old_block_number: U256,
        _old_min: U256,
        _old_max: U256,
        _left_child: Hash,
        _right_child: Hash,
        _old_rows_tree_hash: Hash,
        _extraction_proof: Vec<u8>,
        _rows_tree_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}
