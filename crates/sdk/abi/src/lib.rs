//! Common crate for ABI-encoded types.

use alloy::{
    primitives::{keccak256, Address, FixedBytes},
    sol,
    sol_types::{SolType, SolValue},
};

/// Params for ABI encoded job input.
#[derive(Clone)]
pub struct JobParams<'a> {
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: [u8; 20],
    pub onchain_input: &'a [u8],
    pub program_id: &'a [u8],
    pub offchain_input_hash: [u8; 32],
}

sol! {
    /// The payload that gets signed by the user/app for an offchain job request.
     #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct OffchainJobRequest {
        /// Nonce of the job request for this consumer.
        uint64 nonce;
        /// Max cycles for the job execution.
        uint64 max_cycles;
        /// Consumer to send result to.
        address consumer;
        /// Program ID to execute.
        bytes program_id;
        /// Onchain input for program.
        bytes onchain_input;
        /// Hash of offchain input for program (not posted onchain).
        bytes32 offchain_input_hash;
    }

    /// Onchain input passed to zkVM program for stateful jobs
    struct StatefulAppOnchainInput {
        bytes32 input_state_root;
        bytes onchain_input;
    }

    /// Returned by zkVM program as the result for stateful jobs
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct StatefulAppResult {
        bytes32 output_state_root;
        bytes result;
    }

    /// The payload that gets signed to signify that the zkvm executor has faithfully
    /// executed the job. Also the result payload the job manager contract expects.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct ResultWithMetadata {
        /// Job ID.
        bytes32 job_id;
        /// Hash of onchain input passed to zkVM program for this job.
        bytes32 onchain_input_hash;
        /// Max cycles for the job.
        uint64 max_cycles;
        /// Program ID of program being executed.
        bytes program_id;
        /// Raw output (result) from zkVM program.
        bytes raw_output;
    }

    /// The payload that gets signed to signify that the zkvm executor has faithfully
    /// executed the offchian job. Also the result payload the job manager contract expects.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct OffchainResultWithMetadata {
        /// Job ID.
        bytes32 job_id;
        /// Hash of onchain input passed to zkVM program for this job.
        bytes32 onchain_input_hash;
        /// Hash of offchain input passed to zkVM program for this job.
        bytes32 offchain_input_hash;
        /// Max cycles for the job.
        uint64 max_cycles;
        /// Program ID of program being executed.
        bytes program_id;
        /// Raw output (result) from zkVM program.
        bytes raw_output;
        /// Commitments to the blobs that will contain the offchain input.
        bytes32[] versioned_blob_hashes;
    }
}

/// Returns an ABI-encoded offchain job request. This ABI-encoded response will be
/// signed by the entity sending the job request (user, app, authorized third-party, etc.).
pub fn abi_encode_offchain_job_request(job: JobParams) -> Vec<u8> {
    OffchainJobRequest {
        nonce: job.nonce,
        max_cycles: job.max_cycles,
        consumer: job.consumer_address.into(),
        program_id: job.program_id.to_vec().into(),
        onchain_input: job.onchain_input.to_vec().into(),
        offchain_input_hash: job.offchain_input_hash.into(),
    }
    .abi_encode()
}

/// Returns the job ID hash for a given nonce and consumer address.
pub fn get_job_id(nonce: u64, consumer: Address) -> [u8; 32] {
    keccak256(abi_encode_nonce_and_consumer(nonce, consumer)).into()
}

/// Job nonce and consumer contract address. Used for deriving a job ID.
pub type NonceAndConsumer = sol! {
    tuple(uint64, address)
};

/// Convenience method to abi encode [`NonceAndConsumer`].
pub fn abi_encode_nonce_and_consumer(nonce: u64, consumer: Address) -> Vec<u8> {
    NonceAndConsumer::abi_encode_packed(&(nonce, consumer))
}

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(
    job_id: [u8; 32],
    onchain_input_hash: FixedBytes<32>,
    max_cycles: u64,
    program_id: &[u8],
    raw_output: &[u8],
) -> Vec<u8> {
    ResultWithMetadata {
        job_id: job_id.into(),
        onchain_input_hash,
        max_cycles,
        program_id: program_id.to_vec().into(),
        raw_output: raw_output.to_vec().into(),
    }
    .abi_encode()
}

/// Returns an ABI-encoded offchain result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_offchain_result_with_metadata(
    job_id: [u8; 32],
    onchain_input_hash: FixedBytes<32>,
    offchain_input_hash: FixedBytes<32>,
    max_cycles: u64,
    program_id: &[u8],
    raw_output: &[u8],
    versioned_blob_hashes: Vec<FixedBytes<32>>,
) -> Vec<u8> {
    OffchainResultWithMetadata {
        job_id: job_id.into(),
        onchain_input_hash,
        offchain_input_hash,
        max_cycles,
        program_id: program_id.to_vec().into(),
        raw_output: raw_output.to_vec().into(),
        versioned_blob_hashes,
    }
    .abi_encode()
}
