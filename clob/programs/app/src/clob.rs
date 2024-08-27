//! ZKVM program for running the tick

use alloy_sol_types::SolType;
use clob_core::{
    api::{Request, ClobProgramOutput},
    zkvm_stf, ClobState,
};
use risc0_zkvm::guest::env;

fn main() {
    let state_len: u32 = env::read();
    let mut state_buf = vec![0; state_len as usize];
    env::read_slice(&mut state_buf);
    let state: ClobState = borsh::from_slice(&state_buf).expect("todo");

    let requests_len: u32 = env::read();
    let mut requests_buf = vec![0; requests_len as usize];
    env::read_slice(&mut requests_buf);
    let requests: Vec<Request> = borsh::from_slice(&requests_buf).expect("todo");

    let clob_program_output = zkvm_stf(requests, state);

    let abi_encoded = ClobProgramOutput::abi_encode(&clob_program_output);

    env::commit_slice(&abi_encoded);
}
