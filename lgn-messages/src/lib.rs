#![feature(generic_const_exprs)]
pub mod types;

pub type BlockNr = u64;
pub type TableId = u64;
pub type TableHash = u64;
pub type ChainId = u64;
pub type Proof = Vec<u8>;
pub type QueryId = String;
pub type RowKeyId = String;
