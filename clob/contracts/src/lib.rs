//! Contracts bindings and abi types.

use alloy::sol_types::SolType;

/// The payload that gets signed by the user/app for an offchain job request.
///
/// tuple(Nonce,MaxCycles,Consumer,ProgramID,ProgramInput)
pub type OffchainJobRequest = alloy::sol! {
    tuple(uint64,uint64,address,bytes,bytes)
};

/// Params for ABI encoded job input.
// TODO: make these borrowed values for efficiency.
pub struct JobParams {
    pub nonce: u64,
    pub max_cycles: u64,
    pub consumer_address: [u8; 20],
    pub program_input: Vec<u8>,
    pub program_id: Vec<u8>,
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

/// `ClobConsumer.sol` bindings
pub mod clob_consumer {
    alloy::sol! {
      #[sol(rpc)]
      ClobConsumer,
      "../../contracts/out/ClobConsumer.sol/ClobConsumer.json"
    }
}
