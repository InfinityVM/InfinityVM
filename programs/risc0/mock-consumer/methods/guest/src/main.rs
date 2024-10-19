use alloy::primitives::aliases::U256;
use alloy::primitives::Address;
use alloy::sol_types::SolValue;
use risc0_zkvm::guest::env;

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let start_idx = 32 - 20;
    let address = Address::from_slice(&onchain_input_buf[start_idx..]);

    // Do some very complicated aura points based math to derive
    // a meaningful balance from the address
    let numerator: U256 = address.clone().into_word().into();
    let denominator = U256::try_from(u64::MAX).unwrap();
    let balance = numerator.wrapping_div(denominator);

    let output = (address, balance).abi_encode();

    env::commit_slice(&output);
}