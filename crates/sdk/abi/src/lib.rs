//! Common crate for ABI-encoded types.

use alloy::{sol, sol_types::SolValue};
use db::tables::Job;

/// Errors from ABI encoding
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// invalid address length
    #[error("invalid address length")]
    InvalidAddressLength,
}

/// Params for ABI encoded job input.
#[derive(Clone)]
pub struct JobParams<'a> {
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: [u8; 20],
    pub program_input: &'a [u8],
    pub program_id: &'a [u8],
}

impl<'a> TryFrom<&'a Job> for JobParams<'a> {
    type Error = Error;

    fn try_from(job: &'a Job) -> Result<Self, Error> {
        let consumer_address =
            job.consumer_address.clone().try_into().map_err(|_| Error::InvalidAddressLength)?;

        Ok(JobParams {
            nonce: job.nonce,
            max_cycles: job.max_cycles,
            consumer_address,
            program_input: &job.input,
            program_id: &job.program_id,
        })
    }
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
        /// Program input.
        bytes program_input;
    }

    /// Passed into zkVM program as input for stateful jobs
    struct StatefulProgramInput {
        bytes32 previous_state_hash;
        bytes input;
    }

    /// Returned by zkVM program as the result for stateful jobs
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
        program_input: job.program_input.to_vec().into(),
    }
    .abi_encode()
}
