//! Database tables

use alloy::rlp::bytes;
use borsh::{BorshDeserialize, BorshSerialize};
use proto::Job;
use reth_db::{
    table::{Compress, Decode, Decompress, Encode},
    tables, DatabaseError, TableType, TableViewer,
};
use sha2::{Digest, Sha256};
use std::fmt;

/// Key to a table storing ELFs. The first byte of the key is the vm type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ElfKey(pub [u8; 32]);

impl ElfKey {
    /// New [Self]
    pub(crate) fn new(verifying_key: &[u8]) -> Self {
        let inner: [u8; 32] = Sha256::digest(verifying_key).into();

        Self(inner)
    }
}

impl Encode for ElfKey {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for ElfKey {
    fn decode<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        let inner: [u8; 32] = value.as_ref().try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

/// Storage format for elf files
#[derive(Debug, BorshSerialize, BorshDeserialize, serde::Serialize)]
pub struct ElfWithMeta {
    /// The type of vm
    pub vm_type: u8,
    /// The ELF file
    pub elf: Vec<u8>,
}

impl Compress for ElfWithMeta {
    type Compressed = Vec<u8>;
    fn compress_to_buf<B: bytes::BufMut + AsMut<[u8]>>(self, dest: &mut B) {
        let src = borsh::to_vec(&self).expect("borsh serialize works. qed.");
        dest.put(&src[..])
    }
}

impl Decompress for ElfWithMeta {
    fn decompress<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        borsh::from_slice(value.as_ref()).map_err(|_| DatabaseError::Decode)
    }
}

reth_db::tables! {
    /// Stores Elf files
    table ElfTable<Key = ElfKey, Value = ElfWithMeta>;
    /// Stores jobs
    table JobTable<Key = u32, Value = Job>;
    /// Store completed jobs that failed to broadcast
    table BroadcastJobRetryTable<Key = u32, Value = Job>;
    /// Stores jobs that failed to relay
    table RelayerRetryTable<Key = u32, Value = Job>;
}
