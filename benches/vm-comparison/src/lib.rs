//! VM speed benchmark binary

use borsh::{BorshDeserialize, BorshSerialize};
use ivm_zkvm::{Sp1, Zkvm};

const HIGH_CYCLE_LIMIT: u64 = 1_000_000_000;

/// Input parameters for the intensity test
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct IntensityInput {
    /// Number of hash rounds to perform
    pub hash_rounds: u32,
}

/// Benchmarks SP1 VM execution time for different workload intensities
pub fn compare_vms(sp1_elf: &[u8], intensities: &[u32]) {
    for &intensity in intensities {
        let input = IntensityInput { hash_rounds: intensity };
        let input_bytes = borsh::to_vec(&input).unwrap();

        let start = std::time::Instant::now();
        let _sp1_result = Sp1.execute(sp1_elf, &input_bytes, &[], HIGH_CYCLE_LIMIT).unwrap();
        let sp1_time = start.elapsed();

        println!("\nIntensity (hash rounds): {}", intensity);
        println!("SP1 time: {:?}", sp1_time);
    }
}
