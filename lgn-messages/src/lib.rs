use alloy::primitives::FixedBytes;

pub mod routing;
pub mod types;

pub type BlockNr = u64;
pub type TableId = u64;
pub type TableHash = u64;
pub type ChainId = u64;
pub type QueryId = String;
pub type RowKeyId = String;
pub type Identifier = u64;

/// A keyed payload contains a bunch of bytes accompanied by a storage index
pub type KeyedPayload = (String, Vec<u8>);

/// Identifier for a merkle patricia tree node.
///
/// This type is versioned by the block number, since a node that stores data for
/// a slot `X` can be modified through out the contract's lifetime.
pub type MptNodeVersion = (BlockNr, FixedBytes<32>);
