//! Database tables and persisted types.

use crate::Error;
use alloy::{primitives::utils::keccak256, rlp::bytes};
use ivm_abi::JobParams;
use ivm_eip4844::BlobTransactionSidecar;
use ivm_proto::{JobStatus, JobStatusType, RelayStrategy};
use reth_db::{
    table::{Decode, Encode, TableInfo},
    tables, DatabaseError, TableSet, TableType, TableViewer,
};
use sha2::{Digest, Sha256};
use std::fmt;

macro_rules! impl_compress_decompress {
    ($name:ident) => {
        impl reth_db::table::Compress for $name {
            type Compressed = Vec<u8>;

            fn compress_to_buf<B: bytes::buf::BufMut + AsMut<[u8]>>(&self, dest: &mut B) {
                let src = bincode::serialize(self).expect("failed to serialize into bufmut.");
                // ideally we would use `bincode::serialize_into(dest.writer(), &self)` so we could
                // use the `Write` api, but dest is behind a mutable ref and `.writer` needs
                // to take ownership
                dest.put(&*src)
            }
        }

        impl reth_db::table::Decompress for $name {
            fn decompress(value: &[u8]) -> Result<Self, reth_db::DatabaseError> {
                bincode::deserialize(value.as_ref()).map_err(|_| reth_db::DatabaseError::Decode)
            }
        }
    };
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// Request type for a job
pub enum RequestType {
    /// Onchain job request (originating from contracts)
    Onchain,
    /// Offchain job request. This contains the signature over the ABI-encoded request.
    Offchain(Vec<u8>),
}

/// Job used internally and stored in DB
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Program execution input to be posted onchain
    pub onchain_input: Vec<u8>,
    /// Offchain input (posted to some DA layer instead of onchain)
    pub offchain_input: Vec<u8>,
    /// Contains signature on the offchain job request
    pub request_type: RequestType,
    /// ABI-encoded result of job execution with metadata (can be `ResultWithMetadata` or
    /// `OffchainResultWithMetadata`)
    pub result_with_metadata: Vec<u8>,
    /// The signature of the operator that executed the job
    pub zkvm_operator_signature: Vec<u8>,
    /// The status of the job
    pub status: JobStatus,
    /// Tx hash of relayed result
    pub relay_tx_hash: Vec<u8>,
    /// eip4844 blob transaction sidecar. Only should be present after executing an offchain job.
    pub blobs_sidecar: Option<BlobTransactionSidecar>,
    /// The strategy to use for relaying the job.
    pub relay_strategy: RelayStrategy,
}

impl Job {
    /// Returns true if this job has the relay strategy `Ordered`.
    pub const fn is_ordered(&self) -> bool {
        matches!(self.relay_strategy, RelayStrategy::Ordered)
    }

    /// Returns true if the job has been relayed.
    pub const fn is_relayed(&self) -> bool {
        self.status.status == JobStatusType::Relayed as i32
    }

    /// Returns true if a job is `JobStatusType::Failed`.
    pub const fn is_failed(&self) -> bool {
        self.status.status == JobStatusType::Failed as i32
    }

    /// Returns true if a job is done being executed by has not been relayed.
    pub const fn is_done(&self) -> bool {
        self.status.status == JobStatusType::Done as i32
    }

    /// Returns true if a job has not yet been executed.
    pub const fn is_pending(&self) -> bool {
        self.status.status == JobStatusType::Pending as i32
    }
}

impl<'a> TryFrom<&'a Job> for JobParams<'a> {
    type Error = Error;

    fn try_from(job: &'a Job) -> Result<Self, Error> {
        let consumer_address =
            job.consumer_address.clone().try_into().map_err(|_| Error::InvalidAddressLength)?;
        let offchain_input_hash = keccak256(&job.offchain_input);

        Ok(JobParams {
            nonce: job.nonce,
            max_cycles: job.max_cycles,
            consumer_address,
            onchain_input: &job.onchain_input,
            offchain_input_hash: offchain_input_hash.into(),
            program_id: &job.program_id,
        })
    }
}

/// Key to tables storing job metadata and failed jobs.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct B256Key(pub [u8; 32]);

impl Encode for B256Key {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for B256Key {
    fn decode(value: &[u8]) -> Result<Self, DatabaseError> {
        let inner: [u8; 32] = value.try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

/// Key to a table storing ELFs. The first byte of the key is the vm type.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Sha256Key(pub [u8; 32]);

impl Sha256Key {
    /// New [Self]
    pub fn new(program_id: &[u8]) -> Self {
        let inner: [u8; 32] = Sha256::digest(program_id).into();

        Self(inner)
    }
}

impl Encode for Sha256Key {
    type Encoded = [u8; 32];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for Sha256Key {
    fn decode(value: &[u8]) -> Result<Self, DatabaseError> {
        let inner: [u8; 32] = value.try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

/// Storage format for elf files
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ElfWithMeta {
    /// The type of vm
    pub vm_type: u8,
    /// The ELF file
    pub elf: Vec<u8>,
}

/// Storage format for programs
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProgramWithMeta {
    /// The type of vm
    pub vm_type: u8,
    /// Program serialized with bincode.
    pub program_bytes: Vec<u8>,
}

impl_compress_decompress! { Job }
impl_compress_decompress! { ElfWithMeta }
impl_compress_decompress! { ProgramWithMeta }

/// Key representing an address
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct AddrKey(pub [u8; 20]);
impl Encode for AddrKey {
    type Encoded = [u8; 20];

    fn encode(self) -> Self::Encoded {
        self.0
    }
}

impl Decode for AddrKey {
    fn decode(value: &[u8]) -> Result<Self, DatabaseError> {
        let inner: [u8; 20] = value.try_into().map_err(|_| DatabaseError::Decode)?;

        Ok(Self(inner))
    }
}

reth_db::tables! {
    /// Stores Elf files
    table ElfTable {
        type Key = Sha256Key;
        type Value = ElfWithMeta;
    }

    /// Stores jobs
    table JobTable {
        type Key = B256Key;
        type Value = Job;
    }

    /// Stores failed jobs
    table RelayFailureJobs {
        type Key = B256Key;
        type Value = Job;
    }

    /// Last seen block height
    table LastBlockHeight {
        type Key = u32;
        type Value = u64;
    }

    /// Store programs
    table ProgramTable {
        type Key = Sha256Key;
        type Value = ProgramWithMeta;
    }
}
