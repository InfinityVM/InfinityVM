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

pub use alloy::consensus::{SidecarBuilder, SimpleCoder};
pub use c_kzg::Error as CKzgError;

#[cfg(test)]
mod test {
    use crate::SIMPLE_CODER_MAX_DATA_PER_BLOB;
    use alloy::{
        consensus::{SidecarBuilder, SimpleCoder},
        eips::eip4844::USABLE_BYTES_PER_BLOB,
    };

    #[test]
    fn usable_bytes_per_blob_creates_two_blobs() {
        let len = USABLE_BYTES_PER_BLOB;
        let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8).collect();

        let builder: SidecarBuilder<SimpleCoder> = [data].iter().collect();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 2);
    }

    #[test]
    fn simple_coder_max_data_per_blob_creates_one_blob() {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB;
        let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8).collect();

        let builder: SidecarBuilder<SimpleCoder> = [data].iter().collect();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 1);
    }

    #[test]
    fn simple_coder_max_data_per_blob_plus_one_creates_two_blobs() {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB + 1;
        let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8).collect();

        let builder: SidecarBuilder<SimpleCoder> = [data].iter().collect();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 2);
    }

    #[test]
    fn simple_coder_max_data_per_blob_times_six_creates_six_blobs() {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6;
        let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8).collect();

        let builder: SidecarBuilder<SimpleCoder> = [data].iter().collect();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 6);
    }

    #[test]
    fn simple_coder_max_data_per_blob_times_six_plus_1_creates_seven_blobs() {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6 + 1;
        let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8).collect();

        let builder: SidecarBuilder<SimpleCoder> = [data].iter().collect();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 7);
    }
}
