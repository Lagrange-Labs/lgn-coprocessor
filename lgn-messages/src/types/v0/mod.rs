pub mod groth16;
pub mod preprocessing;
pub mod query;

pub(crate) const STORAGE_QUERY2: &str = "STORAGE_QUERY2";

pub(crate) type ChainId = u64;

pub trait ChainAware {
    fn chain_id(&self) -> ChainId;
}
