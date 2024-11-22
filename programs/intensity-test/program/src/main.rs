#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy::primitives::U256;
use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};

#[derive(BorshDeserialize, BorshSerialize)]
struct IntensityInput {
    hash_rounds: u32,
}

fn do_some_work(intensity: IntensityInput) -> U256 {
    let mut result = U256::ZERO;
    let mut hasher = Sha256::new();

    for _ in 0..intensity.hash_rounds {
        hasher.update(result.to_be_bytes::<32>());
        let hash = hasher.finalize_reset();
        result = result.overflowing_add(U256::from_be_slice(&hash)).0;
    }

    result
}

fn main() {
    let input_bytes = sp1_zkvm::io::read_vec();
    let intensity: IntensityInput = BorshDeserialize::deserialize(&mut &input_bytes[..]).unwrap();
    let result = do_some_work(intensity);

    sp1_zkvm::io::commit_slice(&result.to_be_bytes::<32>());
}
