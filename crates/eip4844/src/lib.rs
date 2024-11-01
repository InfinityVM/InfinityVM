//! Helpers for constructing eip4844 blobs.
//!
//! [Ethereum specs.](https://github.com/ethereum/consensus-specs/blob/86fb82b221474cc89387fa6436806507b3849d88/specs/deneb/polynomial-commitments.md)

use alloy::eips::eip4844::{FIELD_ELEMENTS_PER_BLOB, USABLE_BYTES_PER_BLOB};

/// The first element is used as a header (which contains the length of the data, right padded). The
/// first bit of each field is element is 0.
pub const SIMPLE_CODER_MAX_DATA_PER_BLOB: usize =
    USABLE_BYTES_PER_BLOB - 32 - FIELD_ELEMENTS_PER_BLOB as usize;

/// The max number of blobs in a mainnet block.
pub const MAX_BLOBS_PER_BLOCK: usize = 6;

pub use alloy::consensus::{BlobTransactionSidecar, SidecarBuilder, SimpleCoder};
pub use c_kzg::Error as CKzgError;
