#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::{
    primitives::{Address, U256},
    sol_types::SolValue,
};

// This function is simply doing some mock aura points calculations
// we can adjust the intensity of the calculation by incorporating 
// load testing in the future to SP1. 
fn main() {
    // Read onchain input
    let onchain_input = sp1_zkvm::io::read_vec();

    // Verify input length
    if onchain_input.len() != 32 {
        panic!("Invalid input length: expected 32 bytes");
    }

    // Extract the 20-byte address from the last 20 bytes
    let start_idx = 32 - 20;
    let address_bytes = &onchain_input[start_idx..];

    // Verify address length
    if address_bytes.len() != 20 {
        panic!("Invalid address length: expected 20 bytes");
    }

    let address = Address::from_slice(address_bytes);

    // Return default balance to match mock_raw_output()
    let balance = U256::default();

    // Write output
    let output = (address, balance).abi_encode();
    sp1_zkvm::io::commit_slice(&output);
}
