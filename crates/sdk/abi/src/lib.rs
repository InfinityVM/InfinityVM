//! Common crate for ABI-encoded types.

use alloy::{
    primitives::{keccak256, Address},
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
    pub state_hash: [u8; 32],
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
        /// Hash of state.
        bytes32 state_hash;
    }

    /// Returned by zkVM program as the result for stateful jobs
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct StatefulProgramResult {
        bytes32 next_state_hash;
        bytes result;
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
        state_hash: job.state_hash.into(),
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

/// Continence method to abi encode [`NonceAndConsumer`].
pub fn abi_encode_nonce_and_consumer(nonce: u64, consumer: Address) -> Vec<u8> {
    NonceAndConsumer::abi_encode_packed(&(nonce, consumer))
}
