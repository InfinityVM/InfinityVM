//! ZKVM program for running the tick

use alloy::sol_types::SolType;
use clob_core::{
    api::Request,
    zkvm_stf, ClobState,
};
use abi::StatefulAppResult;
use risc0_zkvm::guest::env;

// TODO: Update the CLOB zkVM program to use state root + merkle proofs instead of the entire state
// [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0u8; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let offchain_input_len: u32 = env::read();
    let mut offchain_input_buf = vec![0u8; offchain_input_len as usize];
    env::read_slice(&mut offchain_input_buf);

    // We combined the requests and state in offchain input, so we need to split them here.
    let requests_len = u32::from_le_bytes(offchain_input_buf[..4].try_into().unwrap()) as usize;
    let requests_borsh = offchain_input_buf[4..4 + requests_len].to_vec();
    let state_borsh = offchain_input_buf[4 + requests_len..].to_vec();

    let requests: Vec<Request> = borsh::from_slice(&requests_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let state: ClobState = borsh::from_slice(&state_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let clob_program_output = zkvm_stf(requests, state);
    let abi_encoded = StatefulAppResult::abi_encode(&clob_program_output);

    env::commit_slice(&abi_encoded);
}
