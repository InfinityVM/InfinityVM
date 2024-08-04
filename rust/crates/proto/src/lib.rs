//! The proto generated server, client, and messages.
//!
//! You can view generated code docs with `make doc`.

// We don't have control over tonic generated code so we ignore the
// lints it complains about
#![allow(clippy::all, clippy::missing_const_for_fn, unreachable_pub)]

use alloy::rlp::bytes;
use reth_db::{
    table::{Compress, Decompress},
    DatabaseError,
};

tonic::include_proto!("coprocessor_node.v1");

/// Reflection
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("descriptor");

impl Compress for Job {
    type Compressed = Vec<u8>;
    fn compress_to_buf<B: bytes::BufMut + AsMut<[u8]>>(self, dest: &mut B) {
        let src = borsh::to_vec(&self).expect("borsh serialize works. qed.");
        dest.put(&src[..])
    }
}

impl Decompress for Job {
    fn decompress<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        borsh::from_slice(value.as_ref()).map_err(|_| DatabaseError::Decode)
    }
}
