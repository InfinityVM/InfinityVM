//! ZKVM program for running the tick

use alloy::sol_types::SolType;
use simple_core::{
    api::Request,
    zkvm_stf, SimpleState,
};
use abi::StatefulProgramResult;
use risc0_zkvm::guest::env;

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let offchain_input_len: u32 = env::read();
    let mut offchain_input_buf = vec![0; offchain_input_len as usize];
    env::read_slice(&mut offchain_input_buf);
    let requests: Vec<Request> = borsh::from_slice(&offchain_input_buf)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let state_len: u32 = env::read();
    let mut state_buf = vec![0; state_len as usize];
    env::read_slice(&mut state_buf);
    let state: SimpleState = borsh::from_slice(&state_buf)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let simple_program_output = zkvm_stf(requests, state);
    let abi_encoded = StatefulProgramResult::abi_encode(&simple_program_output);

    env::commit_slice(&abi_encoded);
}
