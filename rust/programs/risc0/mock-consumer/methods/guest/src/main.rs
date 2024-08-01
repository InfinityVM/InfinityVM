use risc0_zkvm::guest::env;
use std::io::Read;

fn main() {
    // read in data as bytes
    let mut raw_input = vec![];
    env::stdin().read_to_end(&mut raw_input).unwrap();

    let address = Address::abi_decode(&raw_input, false).unwrap();
    let address: U256 = address.into_word().into();

    // Do some very complicated aura points based math to derive
    // a meaningful balance from the address
    let balance = address.wrapping_div(2.into());
    let output = balance.to_be_bytes();

    env::commit_slice(&output);
}
