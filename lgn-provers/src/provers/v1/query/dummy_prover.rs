use lgn_messages::types::v1::query::tasks::MatchingRowInput;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::RowsChunkInput;
use parsil::assembler::DynamicCircuitPis;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation::api::MatchingRow;

use crate::dummy_utils::dummy_proof;
use crate::provers::v1::query::prover::StorageQueryProver;

const PROOF_SIZE: usize = 120;

/// Prover implementation which performs no proving and returns random data as a proof.
pub struct DummyProver;

impl StorageQueryProver for DummyProver {
    fn prove_universal_circuit(
        &self,
        _input: MatchingRowInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_row_chunks(
        &self,
        _input: RowsChunkInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_chunk_aggregation(
        &self,
        _chunks_proofs: &[Vec<u8>],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_non_existence(
        &self,
        _input: NonExistenceInput,
        _pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_aggregated_revelation(
        &self,
        _pis: &DynamicCircuitPis,
        _placeholders: Placeholders,
        _query_proof: Vec<u8>,
        _indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }

    fn prove_tabular_revelation(
        &self,
        _pis: &DynamicCircuitPis,
        _placeholders: Placeholders,
        _preprocessing_proof: Vec<u8>,
        _matching_rows: Vec<MatchingRow>,
        _column_ids: &ColumnIDs,
        _limit: u32,
        _offset: u32,
    ) -> anyhow::Result<Vec<u8>> {
        Ok(dummy_proof(PROOF_SIZE))
    }
}
