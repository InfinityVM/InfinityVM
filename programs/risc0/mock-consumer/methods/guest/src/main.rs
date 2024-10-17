use alloy::primitives::{aliases::U256, Address};
use alloy::sol_types::SolValue;
use risc0_zkvm::guest::env;
use sha2::{Sha256, Digest};

// - "light": for quick, less intensive computations
// - "medium": for balanced performance and complexity
// - "heavy": for more intensive, time-consuming computations
const COMPUTATION_INTENSITY: &str = "light";

fn get_parameters(intensity: &str) -> Vec<(u32, u32)> {
    match intensity.to_lowercase().as_str() {
        "light" => vec![(100, 10)],
        "medium" => vec![(1000, 100)],
        "heavy" => vec![(10000, 1000)],
        _ => vec![(1000, 100)], 
    }
}

fn main() {
    let intensities = get_parameters(COMPUTATION_INTENSITY);

    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let address = Address::from_slice(&onchain_input_buf[12..32]);
    
    let balance = calculate_initial_balance(&address);
    let mut results = Vec::new();

    for (computation_iterations, hash_rounds) in intensities {
        let final_balance = perform_hash_rounds(perform_expensive_computation(balance, computation_iterations), hash_rounds);
        results.push(final_balance);
    }

    env::commit_slice(&(address, results).abi_encode());
}

fn calculate_initial_balance(address: &Address) -> U256 {
    U256::from(address.as_slice().iter().enumerate().fold(0u128, |acc, (i, &byte)| {
        acc.saturating_add((byte as u128).pow(4) * (i as u128 + 1).pow(3))
    }))
}

fn perform_expensive_computation(initial: U256, iterations: u32) -> U256 {
    (0..iterations).fold(initial, |acc, i| { 
        acc.saturating_add(acc.saturating_mul(U256::from((i as u64 + 1).pow(2))))
    })
}

fn perform_hash_rounds(input: U256, rounds: u32) -> U256 {
    let mut hasher = Sha256::new();
    let mut hash_result = input.to_be_bytes::<32>().to_vec();
    
    for _ in 0..rounds {
        hasher.update(&hash_result);
        hash_result = hasher.finalize_reset().to_vec();
    }

    U256::from_be_slice(&hash_result)
}
