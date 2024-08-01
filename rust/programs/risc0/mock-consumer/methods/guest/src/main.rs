use alloy::primitives::aliases::U256;
use alloy::primitives::Address;
use alloy::sol_types::SolType;
use alloy::sol_types::SolValue;
use risc0_zkvm::guest::env;
use std::io::Read;

fn main() {
    // read in data as bytes
    let mut raw_input = vec![];
    env::stdin().read_to_end(&mut raw_input).unwrap();

    let address = Address::from_slice(&raw_input[..20]);
    let address: U256 = address.into_word().into();

    // Do some very complicated aura points based math to derive
    // a meaningful balance from the address
    let two = U256::try_from(2).unwrap();
    let balance = address.wrapping_div(two);
    let output: [u8; 32] = balance.to_be_bytes();

    env::commit_slice(&output);
}
