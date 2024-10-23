//! Helpers for constructing eip4844 blobs.
//!
//! ref: https://github.com/ethereum/consensus-specs/blob/86fb82b221474cc89387fa6436806507b3849d88/specs/deneb/polynomial-commitments.md
// ref: https://github.com/paradigmxyz/reth/blob/bc43613be35f316304f7ce4a2225855ac26b5923/crates/primitives/src/transaction/sidecar.rs#L254-L256
//

use alloy::{
    consensus::{EnvKzgSettings, SidecarBuilder, SimpleCoder, },
    eips::eip4844::{USABLE_BYTES_PER_BLOB, FIELD_ELEMENTS_PER_BLOB},
};

// The first element is used as a header (which contains the length of the data, right padded). The
// first bit of each field is element is 0.
const SIMPLE_CODER_MAX_DATA_PER_BLOB: usize = USABLE_BYTES_PER_BLOB - 32 - FIELD_ELEMENTS_PER_BLOB as usize;


#[cfg(test)]
mod test {
    use crate::SIMPLE_CODER_MAX_DATA_PER_BLOB;
    use alloy::{
        consensus::{SidecarBuilder, SidecarCoder, SimpleCoder},
        eips::eip4844::USABLE_BYTES_PER_BLOB,
    };
    use c_kzg::{BYTES_PER_BLOB, BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB};

    fn four_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 3 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    fn seven_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    

    #[test]
    fn simple_coder() {
        let four_blobs = vec![four_blob_vec()];
        let builder = four_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
        let x = USABLE_BYTES_PER_BLOB;

        let four = builder.build().unwrap();
        assert_eq!(four.blobs.len(), 4);

        let raw_data = vec![seven_blob_vec()];
        let builder = seven_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
        let built = builder.build().unwrap();
        assert_eq!(built.blobs.len(), 7);

        let c = SimpleCoder {};
        let data = c.decode_all(built.blobs).unwrap();
    }

    #[test]
    fn check_understanding() {
        assert_eq!(FIELD_ELEMENTS_PER_BLOB, 4096);
        assert_eq!(BYTES_PER_FIELD_ELEMENT, 32);
    }
}
