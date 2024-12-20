use lgn_messages::types::v1::query::tasks::MatchingRowInput;
use lgn_messages::types::v1::query::tasks::NonExistenceInput;
use lgn_messages::types::v1::query::tasks::RowsChunkInput;
use parsil::assembler::DynamicCircuitPis;
use verifiable_db::query::computational_hash_ids::ColumnIDs;
use verifiable_db::query::universal_circuit::universal_circuit_inputs::Placeholders;
use verifiable_db::revelation::api::MatchingRow;

pub trait StorageQueryProver
{
    fn prove_universal_circuit(
        &self,
        input: MatchingRowInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_row_chunks(
        &self,
        input: RowsChunkInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_chunk_aggregation(
        &self,
        chunks_proofs: &[Vec<u8>],
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_non_existence(
        &self,
        input: NonExistenceInput,
        pis: &DynamicCircuitPis,
    ) -> anyhow::Result<Vec<u8>>;

    fn prove_aggregated_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        query_proof: Vec<u8>,
        indexing_proof: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>>;

    #[allow(clippy::too_many_arguments)]
    fn prove_tabular_revelation(
        &self,
        pis: &DynamicCircuitPis,
        placeholders: Placeholders,
        preprocessing_proof: Vec<u8>,
        matching_rows: Vec<MatchingRow>,
        column_ids: &ColumnIDs,
        limit: u32,
        offset: u32,
    ) -> anyhow::Result<Vec<u8>>;
}
