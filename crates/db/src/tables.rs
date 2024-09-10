//! Database tables

use alloy::{
    primitives::{utils::keccak256, Address},
    rlp::bytes,
    sol,
    sol_types::SolType,
};

use crate::Error;
use abi::JobParams;
use borsh::{BorshDeserialize, BorshSerialize};
use proto::JobStatus;
use reth_db::{
    table::{Decode, Encode},
    tables, DatabaseError, TableType, TableViewer,
};
use sha2::{Digest, Sha256};
use std::fmt;

macro_rules! impl_compress_decompress {
    ($name:ident) => {
        impl reth_db::table::Compress for $name {
            type Compressed = Vec<u8>;

            fn compress_to_buf<B: bytes::buf::BufMut + AsMut<[u8]>>(self, dest: &mut B) {
                let src = borsh::to_vec(&self).expect("borsh serialize works. qed.");
                dest.put(&src[..])
            }
        }

        impl reth_db::table::Decompress for $name {
            fn decompress<B: AsRef<[u8]>>(value: B) -> Result<Self, reth_db::DatabaseError> {
                borsh::from_slice(value.as_ref()).map_err(|_| reth_db::DatabaseError::Decode)
            }
        }
    };
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, BorshSerialize, BorshDeserialize)]
/// Request type for a job
pub enum RequestType {
    /// Onchain job request (originating from contracts)
    Onchain,
    /// Offchain job request. This contains the signature over the ABI-encoded request.
    Offchain(Vec<u8>),
}

/// Job used internally and stored in DB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Job {
    /// The job ID (hash of nonce and consumer address)
    pub id: [u8; 32],
    /// Nonce of the job request for a given consumer contract
    pub nonce: u64,
    /// CPU cycle limit for job execution
    pub max_cycles: u64,
    /// Address of consumer contract to receive this result. The address is 20 bytes (no zero
    /// padding).
    pub consumer_address: Vec<u8>,
    /// The ZK program verification key
    pub program_id: Vec<u8>,
    /// Program execution input
    pub input: Vec<u8>,
    /// Program state
    pub program_state: Vec<u8>,
    /// Signature on the offchain job request
    pub request_type: RequestType,
    /// ABI-encoded result of job execution with metadata
    /// tuple(JobID,ProgramInputHash,MaxCycles,ProgramID,RawOutput)
    pub result_with_metadata: Vec<u8>,
    /// The signature of the operator that executed the job
    pub zkvm_operator_signature: Vec<u8>,
    /// The status of the job
    pub status: JobStatus,
}

impl_compress_decompress! { Job }

impl<'a> TryFrom<&'a Job> for JobParams<'a> {
    type Error = Error;

    fn try_from(job: &'a Job) -> Result<Self, Error> {
        let consumer_address =
            job.consumer_address.clone().try_into().map_err(|_| Error::InvalidAddressLength)?;

        Ok(JobParams {
            nonce: job.nonce,
            max_cycles: job.max_cycles,
            consumer_address,
            onchain_input: &job.input,
            program_id: &job.program_id,
        })
    }
}

/// Returns the job ID hash for a given nonce and consumer address.
pub fn get_job_id(nonce: u64, consumer: Address) -> [u8; 32] {
    keccak256(abi_encode_nonce_and_consumer(nonce, consumer)).into()
}

type NonceAndConsumer = sol! {
    tuple(uint64, address)
};

fn abi_encode_nonce_and_consumer(nonce: u64, consumer: Address) -> Vec<u8> {
    NonceAndConsumer::abi_encode_packed(&(nonce, consumer))
}

/// Key to tables storing job metadata and failed jobs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct JobID(pub [u8; 32]);

impl Encode for JobID {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for JobID {
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

impl_compress_decompress! { ElfWithMeta }

reth_db::tables! {
    /// Stores Elf files
    table ElfTable<Key = ElfKey, Value = ElfWithMeta>;
    /// Stores jobs
    table JobTable<Key = JobID, Value = Job>;
    /// Stores failed jobs
    table RelayFailureJobs<Key = JobID, Value = Job>;
    /// Last seen block height
    table LastBlockHeight<Key = u32, Value = u64>;
}
