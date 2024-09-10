//! Common crate for ABI-encoded types.

use alloy::{sol, sol_types::SolValue};

/// Params for ABI encoded job input.
#[derive(Clone)]
pub struct JobParams<'a> {
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: [u8; 20],
    pub onchain_input: &'a [u8],
    pub program_id: &'a [u8],
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
        /// Program input posted onchain.
        bytes onchain_input;
    }

    /// Passed into zkVM program as input for stateful jobs. This is just the
    /// input posted onchain. There is additional offchain input (program state).
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct StatefulProgramInput {
        bytes32 previous_state_hash;
        bytes input;
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
    }
    .abi_encode()
}
