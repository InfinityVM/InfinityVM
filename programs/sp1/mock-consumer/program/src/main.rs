#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::{
    primitives::{Address, U256},
    sol_types::SolValue,
};

fn main() {
    // Read onchain input
    let onchain_input = sp1_zkvm::io::read_vec();

    // Extract the 20-byte address from the last 20 bytes
    let start_idx = 32 - 20;
    let address = Address::from_slice(&onchain_input[start_idx..]);

    // Do some very complicated aura points based math to derive
    // a meaningful balance from the address
    let numerator: U256 = address.clone().into_word().into();
    let denominator = U256::try_from(u64::MAX).unwrap();
    let balance = numerator.wrapping_div(denominator);

    // Write output
    let output = (address, balance).abi_encode();
    sp1_zkvm::io::commit_slice(&output);
}
