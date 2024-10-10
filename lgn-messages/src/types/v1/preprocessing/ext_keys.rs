use crate::types::v1::preprocessing::ext_tasks::MptNodeVersion;
use crate::types::v1::preprocessing::KEYS_PREPROCESSING_PREFIX;
use alloy_primitives::Address;
use object_store::path::Path;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

type TableId = u64;

type BlockNr = u64;

const MPT_VARIABLE_PREFIX: &str = "MPT_VARIABLE";

const MPT_LENGTH_PREFIX: &str = "MPT_LENGTH";

const CONTRACT_PREFIX: &str = "CONTRACT";

const BLOCK_PREFIX: &str = "EXT_BLOCK";

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
                // Example: V1_PREPROCESSING/1/MPT_VARIABLE/1/0x1234_1
                write!(
                    f,
                    "{}/{}/{}/{}/{}",
                    KEYS_PREPROCESSING_PREFIX,
                    table_id,
                    MPT_VARIABLE_PREFIX,
                    node_version.0,
                    node_version.1
                )
            }
            ProofKey::MptLength(table_id, block_nr) => {
                // Example: V1_PREPROCESSING/1/MPT_LENGTH/1
                write!(
                    f,
                    "{}/{}/{}/{}",
                    KEYS_PREPROCESSING_PREFIX, table_id, MPT_LENGTH_PREFIX, block_nr
                )
            }
            ProofKey::Contract(address, block_nr) => {
                // Example: V1_PREPROCESSING/CONTRACT/0x1234/1
                write!(
                    f,
                    "{}/{}/{}/{}",
                    KEYS_PREPROCESSING_PREFIX, CONTRACT_PREFIX, address, block_nr
                )
            }
            ProofKey::Block(block_nr) => {
                // Example: V1_PREPROCESSING/EXT_BLOCK/1
                write!(
                    f,
                    "{}/{}/{}",
                    KEYS_PREPROCESSING_PREFIX, BLOCK_PREFIX, block_nr
                )
            }
            ProofKey::FinalExtraction(table_id, block_nr) => {
                // Example: V1_PREPROCESSING/1/FINAL_EXTRACTION/1
                write!(
                    f,
                    "{}/{}/{}/{}",
                    KEYS_PREPROCESSING_PREFIX, table_id, FINAL_EXTRACTION_PREFIX, block_nr
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
