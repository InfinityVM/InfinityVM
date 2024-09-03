//! Common crate for ABI-encoded types.

use alloy::sol_types::SolType;
use db::tables::Job;

/// Errors from ABI encoding
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// invalid address length
    #[error("invalid address length")]
    InvalidAddressLength,
}

/// The payload that gets signed by the user/app for an offchain job request.
///
/// tuple(Nonce,MaxCycles,Consumer,ProgramID,ProgramInput)
pub type OffchainJobRequest = alloy::sol! {
    tuple(uint64,uint64,address,bytes,bytes)
};

/// Params for ABI encoded job input.
// TODO: make these borrowed values for efficiency.
#[derive(Clone)]
pub struct JobParams {
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: [u8; 20],
    pub program_input: Vec<u8>,
    pub program_id: Vec<u8>,
}

impl TryFrom<Job> for JobParams {
    type Error = Error;

    fn try_from(job: Job) -> Result<Self, Error> {
        let consumer_address =
            job.consumer_address.try_into().map_err(|_| Error::InvalidAddressLength)?;

        Ok(JobParams {
            nonce: job.nonce,
            max_cycles: job.max_cycles,
            consumer_address,
            program_input: job.input,
            program_id: job.program_id,
        })
    }
}

/// Returns an ABI-encoded offchain job request. This ABI-encoded response will be
/// signed by the entity sending the job request (user, app, authorized third-party, etc.).
pub fn abi_encode_offchain_job_request(job: JobParams) -> Vec<u8> {
    OffchainJobRequest::abi_encode_params(&(
        job.nonce,
        job.max_cycles,
        job.consumer_address,
        job.program_id,
        job.program_input,
    ))
}
