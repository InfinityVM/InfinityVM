#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::sol_types::SolType;
use clob_core::{api::Request, zkvm_stf, ClobState};
use ivm_abi::StatefulAppResult;

fn main() {
    // Read onchain input
    let onchain_input = sp1_zkvm::io::read_vec();

    // Read offchain input
    let offchain_input = sp1_zkvm::io::read_vec();

    // Split combined requests and state
    let requests_len = u32::from_le_bytes(offchain_input[..4].try_into().unwrap()) as usize;
    let requests_borsh = offchain_input[4..4 + requests_len].to_vec();
    let state_borsh = offchain_input[4 + requests_len..].to_vec();

    let requests: Vec<Request> = borsh::from_slice(&requests_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let state: ClobState = borsh::from_slice(&state_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let clob_program_output = zkvm_stf(requests, state);
    
    sp1_zkvm::io::commit_slice(&StatefulAppResult::abi_encode(&clob_program_output));
} 