use alloy::primitives::aliases::U256;
use alloy::primitives::Address;
use alloy::sol_types::SolValue;
use risc0_zkvm::guest::env;
use std::io::Read;

fn main() {
    // read in data as bytes
    let mut raw_input = vec![];
    // N.B. reads in one "word" (32 bytes) at a time
    env::stdin().read_to_end(&mut raw_input).unwrap();

    let start_idx = 32 - 20;
    let address = Address::from_slice(&raw_input[start_idx..]);

    // Do some very complicated aura points based math to derive
    // a meaningful balance from the address
    let numerator: U256 = address.clone().into_word().into();
    let denominator = U256::try_from(u64::MAX).unwrap();
    let balance = numerator.wrapping_div(denominator);

    let output = (balance, address).abi_encode();

    env::commit_slice(&output);
}
