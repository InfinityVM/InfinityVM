//! Database tables

use alloy::rlp::bytes;
use borsh::{BorshDeserialize, BorshSerialize};
use proto::JobStatus;
use reth_db::{
    table::{Compress, Decode, Decompress, Encode},
    tables, DatabaseError, TableType, TableViewer,
};
use sha2::{Digest, Sha256};
use std::fmt;

// Job used internally and stored in DB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Job {
    pub id: [u8; 32],
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: Vec<u8>,
    pub program_id: Vec<u8>,
    pub input: Vec<u8>,
    pub request_signature: Vec<u8>,
    pub result: Vec<u8>,
    pub zkvm_operator_address: Vec<u8>,
    pub zkvm_operator_signature: Vec<u8>,
    pub status: JobStatus,
}

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

/// Key to tables storing job metadata and failed jobs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct JobKey(pub [u8; 32]);

impl Encode for JobKey {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for JobKey {
    fn decode<B: AsRef<[u8]>>(value: B) -> Result<Self, DatabaseError> {
        let inner: [u8; 32] = value.as_ref().try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

/// Key to a table storing ELFs. The first byte of the key is the vm type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct ElfKey(pub [u8; 32]);

impl ElfKey {
    /// New [Self]
    pub(crate) fn new(program_id: &[u8]) -> Self {
        let inner: [u8; 32] = Sha256::digest(program_id).into();

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
    table JobTable<Key = JobKey, Value = Job>;
    /// Stores failed jobs
    table RelayFailureJobs<Key = JobKey, Value = Job>;
}
