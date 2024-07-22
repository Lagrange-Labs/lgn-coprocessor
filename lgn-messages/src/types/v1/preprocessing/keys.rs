use std::fmt::{Display, Formatter};

use ethers::prelude::Address;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};

use crate::types::v0::preprocessing::keys::BlockNr;
use crate::types::v1::preprocessing::task::{
    ExtractionType, MptNodeVersion, MptType, WorkerTaskType,
};
use crate::types::TaskType;

type TableId = u64;

const PREPROCESSING_PREFIX: &str = "V1_PREPROCESSING";

const MPT_VARIABLE_PREFIX: &str = "MPT_VARIABLE";

const MPT_LENGTH_PREFIX: &str = "MPT_LENGTH";

const CONTRACT_PREFIX: &str = "CONTRACT";

const BLOCK_PREFIX: &str = "BLOCK";

const FINAL_EXTRACTION_PREFIX: &str = "FINAL_EXTRACTION";
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Serialize, Deserialize)]
pub enum ProofKey {
    /// Indicates the root location of `PublicParams`.
    PublicParams,

    /// Indicates the location of `MPT` proof tree node.
    MptVariable(TableId, MptNodeVersion),

    /// Indicates the location of Length slot proof.
    MptLength(TableId, BlockNr),

    /// Indicates the location of Contract proof.
    Contract(Address, BlockNr),

    /// Indicates the location of Block proof.
    Block(BlockNr),

    /// Indicates the location of FinalExtraction proof.
    FinalExtraction(TableId, BlockNr),
}

impl Display for ProofKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofKey::PublicParams => {
                write!(f, "PublicParams_v1")
            }
            ProofKey::MptVariable(table_id, node_version) => {
                // Example: V1_PREPROCESSING_1_MPT_VARIABLE_1_0x1234_1
                write!(
                    f,
                    "{}_{}_{}_{}_{}",
                    PREPROCESSING_PREFIX,
                    table_id,
                    MPT_VARIABLE_PREFIX,
                    node_version.0,
                    node_version.1
                )
            }
            ProofKey::MptLength(table_id, block_nr) => {
                // Example: V1_PREPROCESSING_1_MPT_LENGTH_1_0x1234_1
                write!(
                    f,
                    "{}_{}_{}_{}",
                    PREPROCESSING_PREFIX, table_id, MPT_LENGTH_PREFIX, block_nr
                )
            }
            ProofKey::Contract(address, block_nr) => {
                // Example: V1_PREPROCESSING_CONTRACT_0x1234_1
                write!(
                    f,
                    "{}_{}_{}_{}",
                    PREPROCESSING_PREFIX, CONTRACT_PREFIX, address, block_nr
                )
            }
            ProofKey::Block(block_nr) => {
                // Example: V1_PREPROCESSING_1_MPT_BLOCK_1
                write!(f, "{}_{}_{}", PREPROCESSING_PREFIX, BLOCK_PREFIX, block_nr)
            }
            ProofKey::FinalExtraction(table_id, block_nr) => {
                // Example: V1_PREPROCESSING_1_FINAL_EXTRACTION_1
                write!(
                    f,
                    "{}_{}_{}_{}",
                    PREPROCESSING_PREFIX, table_id, FINAL_EXTRACTION_PREFIX, block_nr
                )
            }
        }
    }
}

impl From<ProofKey> for Path {
    fn from(key: ProofKey) -> Self {
        Path::from(key.to_string())
    }
}

impl From<ProofKey> for String {
    fn from(key: ProofKey) -> Self {
        key.to_string()
    }
}

impl From<&TaskType> for ProofKey {
    fn from(task: &TaskType) -> Self {
        match task {
            TaskType::V1Preprocessing(tt) => match &tt.task_type {
                WorkerTaskType::Extraction(ext) => match ext {
                    ExtractionType::MptExtraction(mpt) => {
                        let node_version = (mpt.block_nr, mpt.node_hash);
                        match &mpt.mpt_type {
                            MptType::MappingLeaf(_) => {
                                ProofKey::MptVariable(mpt.table_id, node_version)
                            }
                            MptType::MappingBranch(_) => {
                                ProofKey::MptVariable(mpt.table_id, node_version)
                            }
                            MptType::VariableLeaf(_) => {
                                ProofKey::MptVariable(mpt.table_id, node_version)
                            }
                            MptType::VariableBranch(_) => {
                                ProofKey::MptVariable(mpt.table_id, node_version)
                            }
                        }
                    }
                    ExtractionType::LengthExtraction(length) => {
                        ProofKey::MptLength(length.table_id, length.block_nr)
                    }
                    ExtractionType::ContractExtraction(contract) => {
                        ProofKey::Contract(contract.contract, contract.block_nr)
                    }
                    ExtractionType::BlockExtraction(_) => ProofKey::Block(tt.block_nr),
                    ExtractionType::FinalExtraction(fe) => {
                        ProofKey::FinalExtraction(fe.table_id, fe.block_nr)
                    }
                },
                _ => ProofKey::PublicParams,
            },
            _ => {
                unimplemented!("TaskType not implemented");
            }
        }
    }
}
