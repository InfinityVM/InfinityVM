//! ZKVM program for running the tick

use alloy::primitives::utils::keccak256;
use alloy_sol_types::SolType;
use clob_core::{
    api::{Request, ClobProgramInput, ClobProgramOutput},
    zkvm_stf, ClobState,
};
use risc0_zkvm::guest::env;

fn main() {
    let state_len: u32 = env::read();
    let mut state_buf = vec![0; state_len as usize];
    env::read_slice(&mut state_buf);
    let state: ClobState = borsh::from_slice(&state_buf).expect("todo");

    let input_len: u32 = env::read();
    let mut input_buf = vec![0; input_len as usize];
    env::read_slice(&mut input_buf);
    let input = ClobProgramInput::abi_decode(&input_buf, false).expect("todo");
    // Assert that the provided previous state hash = keccak256 hash of the previous state
    let state_hash = keccak256(&state_buf);
    if state_hash.to_vec() != input.prev_state_hash.to_vec() {
        let empty_output = ClobProgramOutput::abi_encode(&ClobProgramOutput::default());
        env::commit_slice(&empty_output);
        return;
    }

    let requests: Vec<Request> = borsh::from_slice(&input.orders).expect("todo");

    let clob_program_output = zkvm_stf(requests, state);

    let abi_encoded = ClobProgramOutput::abi_encode(&clob_program_output);

    env::commit_slice(&abi_encoded);
}
