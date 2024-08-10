#![feature(generic_const_exprs)]

use std::str::FromStr;

use alloy::eips::BlockId;
use alloy::primitives::{Address, Bytes, U256};
use alloy::rpc::types::Block;
use alloy::{
    eips::BlockNumberOrTag,
    providers::{Provider, ProviderBuilder},
    rpc::types::BlockTransactionsKind,
};
use hex::FromHex;
use mimalloc::MiMalloc;
use mp2_common::eth::{BlockUtil, StorageSlot};
use mp2_v1::api::PublicParameters;
use mp2_v1::values_extraction::{
    identifier_block_column, identifier_for_mapping_key_column, identifier_for_mapping_value_column,
};
use tracing_subscriber::EnvFilter;

use lgn_messages::types::v1::preprocessing::db_tasks::{
    BlockLeafInput, CellLeafInput, DatabaseType, DbBlockType, DbCellType, DbRowType, IndexInputs,
    RowLeafInput,
};
use lgn_messages::types::v1::preprocessing::ext_keys::ProofKey;
use lgn_messages::types::v1::preprocessing::ext_tasks::{
    BlockExtractionInput, Contract, FinalExtraction, FinalExtractionType, MappingBranchInput,
    MappingLeafInput, Mpt, MptType, WorkerTask,
};
use lgn_messages::types::v1::preprocessing::{db_keys, ext_tasks, WorkerTaskType};
use lgn_provers::params::ParamsLoader;
use lgn_provers::provers::v1::preprocessing::euclid_prover::EuclidProver;
use lgn_provers::provers::v1::preprocessing::task::Preprocessing;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// Only for testing purposes

#[tokio::main]
async fn main() {
    test_preprocessing().await;
}

pub(crate) async fn test_preprocessing() {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let address = Address::from_hex("0x2Ea072948819c4aC82a10d5F8067581488351eD9").unwrap();
    let provider = ProviderBuilder::new().on_http("https://holesky-rpc.dev.distributed-query.io/kajdaia-neschastlivaia-cemia-neschastliva-po-svoeiemou".parse().unwrap());

    let slot = StorageSlot::Mapping(vec![0], 1).location();

    let proofs = provider
        .get_proof(address, (&[slot]).to_vec())
        .block_id(BlockNumberOrTag::Number(2106088).into())
        .await
        .unwrap();

    let storage_root = proofs.storage_hash.0.to_vec();

    let contract_nodes = proofs.account_proof.clone();
    let contract_nodes: Vec<Vec<u8>> = contract_nodes.iter().rev().map(|x| x.to_vec()).collect();

    let storage_proof: Vec<Bytes> = proofs.storage_proof[0].proof.clone();
    let storage_proof: Vec<Vec<u8>> = storage_proof.iter().map(|x| x.to_vec()).collect();

    let mpt_leaf_node = storage_proof[1].clone();
    let mpt_branch_node = storage_proof[0].clone();

    let block: Block = provider
        .get_block(
            BlockId::Number(BlockNumberOrTag::Number(2106088)),
            BlockTransactionsKind::Hashes,
        )
        .await
        .unwrap()
        .unwrap();

    let rlp_header = block.rlp();

    let params: PublicParameters = ParamsLoader::prepare_bincode(
        "base_url",
        "/Users/andrussalumets/IdeaProjects/zkmr_params_v1",
        "preprocessing_params.bin",
        "checksum_expected_local_path",
        true,
        true,
    )
    .unwrap();

    let euclid_prover = EuclidProver::new(params);

    let mut preprocessing = Preprocessing::new(euclid_prover);

    let mapping_slot = 1;

    // NFT id = 0
    let mapping_key = U256::ZERO;

    let user_address = Address::from_hex("0xcd82fc81790a8cf5081f026d2219c91be5a497b5").unwrap();
    let mapping_value = U256::from_be_slice(&user_address.into_array());

    let cell_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Database(DatabaseType::Cell(DbCellType::Leaf(CellLeafInput {
            table_id: 0,
            row_id: "".to_string(),
            cell_id: 0,
            identifier: identifier_for_mapping_value_column(mapping_slot, &address),
            value: mapping_value,
        }))),
    };

    let cells_proof = preprocessing.run_inner(cell_task).unwrap();

    let row_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Database(DatabaseType::Row(DbRowType::Leaf(RowLeafInput {
            table_id: 0,
            row_id: "".to_string(),
            identifier: identifier_for_mapping_key_column(mapping_slot, &address),
            value: mapping_key,
            cells_proof_location: None,
            cells_proof,
        }))),
    };

    let rows_proof = preprocessing.run_inner(row_task).unwrap();

    let mpt_leaf_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Extraction(ext_tasks::ExtractionType::MptExtraction(Mpt {
            table_id: 0,
            block_nr: 0,
            node_hash: Default::default(),
            mpt_type: MptType::MappingLeaf(MappingLeafInput {
                key: mapping_key.to_be_bytes_vec(),
                node: mpt_leaf_node,
                slot: mapping_slot as usize,
                contract_address: address,
            }),
        })),
    };

    let mpt_leaf_proof = preprocessing.run_inner(mpt_leaf_task).unwrap();

    let mpt_branch_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Extraction(ext_tasks::ExtractionType::MptExtraction(Mpt {
            table_id: 0,
            block_nr: 0,
            node_hash: Default::default(),
            mpt_type: MptType::MappingBranch(MappingBranchInput {
                node: mpt_branch_node,
                children: vec![],
                children_proofs: vec![mpt_leaf_proof],
            }),
        })),
    };

    let mpt_branch_proof = preprocessing.run_inner(mpt_branch_task).unwrap();

    let contract_proof_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Extraction(ext_tasks::ExtractionType::ContractExtraction(
            Contract {
                block_nr: 0,
                storage_root,
                contract: address,
                nodes: contract_nodes,
            },
        )),
    };

    let contract_proof = preprocessing.run_inner(contract_proof_task).unwrap();

    let ext_block_proof_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Extraction(ext_tasks::ExtractionType::BlockExtraction(
            BlockExtractionInput { rlp_header },
        )),
    };

    let block_proof = preprocessing.run_inner(ext_block_proof_task).unwrap();

    let final_extraction_compound_simple_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Extraction(ext_tasks::ExtractionType::FinalExtraction(
            FinalExtraction {
                table_id: 0,
                block_nr: 0,
                contract: address,
                value_proof_version: (0, Default::default()),
                extraction_type: FinalExtractionType::Simple(true),
                block_proof,
                contract_proof,
                value_proof: mpt_branch_proof,
                length_proof: vec![],
            },
        )),
    };

    let final_extraction_compund_simple_proof = preprocessing
        .run_inner(final_extraction_compound_simple_task)
        .unwrap();

    let block_leaf_input = IndexInputs {
        table_id: 0,
        block_nr: 0,
        inputs: vec![DbBlockType::Leaf(BlockLeafInput {
            table_id: 0,
            block_id: identifier_block_column(),
            extraction_proof_location: ProofKey::PublicParams,
            rows_proof_location: db_keys::ProofKey::Block(0, 0),
            extraction_proof: final_extraction_compund_simple_proof,
            rows_proof,
        })],
    };
    let db_block_task = WorkerTask {
        block_nr: 0,
        chain_id: 0,
        task_type: WorkerTaskType::Database(DatabaseType::Index(block_leaf_input)),
    };

    let db_block_proof = preprocessing.run_inner(db_block_task).unwrap();
}
