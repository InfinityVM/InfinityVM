//! Helpers for constructing eip4844 blobs.
//!
//! ref: <https://github.com/ethereum/consensus-specs/blob/86fb82b221474cc89387fa6436806507b3849d88/specs/deneb/polynomial-commitments.md>
// ref: https://github.com/paradigmxyz/reth/blob/bc43613be35f316304f7ce4a2225855ac26b5923/crates/primitives/src/transaction/sidecar.rs#L254-L256
//

use alloy::eips::eip4844::{FIELD_ELEMENTS_PER_BLOB, USABLE_BYTES_PER_BLOB};

// The first element is used as a header (which contains the length of the data, right padded). The
// first bit of each field is element is 0.
const SIMPLE_CODER_MAX_DATA_PER_BLOB: usize =
    USABLE_BYTES_PER_BLOB - 32 - FIELD_ELEMENTS_PER_BLOB as usize;

#[cfg(test)]
mod test {
    use crate::SIMPLE_CODER_MAX_DATA_PER_BLOB;
    use alloy::{
        consensus::{SidecarBuilder, SimpleCoder},
        eips::eip4844::USABLE_BYTES_PER_BLOB, primitives::U256,
    };
    use c_kzg::{BYTES_PER_FIELD_ELEMENT, FIELD_ELEMENTS_PER_BLOB};

    fn four_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 3 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    fn seven_blob_vec() -> Vec<u8> {
        let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6 + 256;

        (0..len).map(|n| n as u8 % u8::MAX).collect()
    }

    #[test]
    fn usable_bytes_per_blob_creates_two_blobs() {
      let len = USABLE_BYTES_PER_BLOB;
      let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8 ).collect();

      let builder: SidecarBuilder<SimpleCoder> = vec![data].iter().collect();
      let built = builder.build().unwrap();
      assert_eq!(built.blobs.len(), 2);
    }

    #[test]
    fn simple_coder_max_data_per_blob_creates_one_blob() {
      let len = SIMPLE_CODER_MAX_DATA_PER_BLOB;
      let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8 ).collect();

      let builder: SidecarBuilder<SimpleCoder> = vec![data].iter().collect();
      let built = builder.build().unwrap();
      assert_eq!(built.blobs.len(), 1);
    }

    #[test]
    fn simple_coder_max_data_per_blob_plus_one_creates_two_blobs() {
      let len = SIMPLE_CODER_MAX_DATA_PER_BLOB + 1;
      let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8 ).collect();

      let builder: SidecarBuilder<SimpleCoder> = vec![data].iter().collect();
      let built = builder.build().unwrap();
      assert_eq!(built.blobs.len(), 2);
    }

    #[test]
    fn simple_coder_max_data_per_blob_times_six_creates_six_blobs() {
      let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6;
      let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8 ).collect();

      let builder: SidecarBuilder<SimpleCoder> = vec![data].iter().collect();
      let built = builder.build().unwrap();
      assert_eq!(built.blobs.len(), 6);
    }

    #[test]
    fn simple_coder_max_data_per_blob_times_six_plus_1_creates_seven_blobs() {
      let len = SIMPLE_CODER_MAX_DATA_PER_BLOB * 6 + 1;
      let data: Vec<u8> = (0..len).map(|n| (n % u8::MAX as usize) as u8 ).collect();

      let builder: SidecarBuilder<SimpleCoder> = vec![data].iter().collect();
      let built = builder.build().unwrap();
      assert_eq!(built.blobs.len(), 7);
    }

    // https://github.com/alloy-rs/alloy/blob/0361c785e37717a2610cef300cef1e6ebf6cd5ad/crates/eips/src/eip4844/mod.rs#L34
    #[test]
    fn field_element_with_gt_bls_modulus_is_rejected() {
      use alloy::eips::eip4844::BLS_MODULUS;
      use alloy::primitives::U256;
      let u256 = U256::try_from(1_000).unwrap();

      let new_number = BLS_MODULUS + u256;
      let bytes = newm_number.bytes

    }

    // #[test]
    // fn simple_coder() {
    //     let four_blobs = vec![four_blob_vec()];
    //     let builder = four_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
    //     let x = USABLE_BYTES_PER_BLOB;

    //     let four = builder.build().unwrap();
    //     assert_eq!(four.blobs.len(), 4);

    //     let raw_data = vec![seven_blob_vec()];
    //     let builder = seven_blobs.iter().collect::<SidecarBuilder<SimpleCoder>>();
    //     let built = builder.build().unwrap();
    //     assert_eq!(built.blobs.len(), 7);

    //     let c = SimpleCoder {};
    //     let data = c.decode_all(built.blobs).unwrap();
    // }

    #[test]
    fn check_understanding() {
        assert_eq!(FIELD_ELEMENTS_PER_BLOB, 4096);
        assert_eq!(BYTES_PER_FIELD_ELEMENT, 32);
    }
}
