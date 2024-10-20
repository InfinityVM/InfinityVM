use risc0_zkvm::guest::env;
use alloy::primitives::{Address, U256};
use alloy::sol_types::SolValue;
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Sha256, Digest};

#[derive(BorshDeserialize, BorshSerialize)]
struct IntensityInput {
    iterations: u32,
    work_per_iteration: u32,
}

fn do_some_work(intensity: IntensityInput) -> U256 {
    let mut result = U256::ZERO;
    let mut hasher = Sha256::new();
    
    for _ in 0..intensity.iterations {
        hasher.update(result.to_be_bytes::<32>());
        let hash = hasher.finalize_reset();
        
        for _ in 0..intensity.work_per_iteration {
            result = result.overflowing_add(U256::from_be_slice(&hash)).0;
            hasher.update(result.to_be_bytes::<32>());
            let intermediate_hash = hasher.finalize_reset();
            result = result ^ U256::from_be_slice(&intermediate_hash);
        }
    }
    
    result
}

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let intensity: IntensityInput = BorshDeserialize::deserialize(&mut &onchain_input_buf[..]).unwrap();
    // Do some very complicated aura points based math to derive
    let result = do_some_work(intensity);

    // Format output for MockConsumer contract
    let mock_user_address = Address::repeat_byte(69);
    let output = (mock_user_address, result);
    let abi_encoded_output = output.abi_encode();

    // Write the encoded output back to the host
    env::commit_slice(&abi_encoded_output);
}
