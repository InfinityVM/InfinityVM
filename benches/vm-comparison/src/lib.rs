//! VM comparison benchmarks for comparing RISC0 and SP1 performance.

use borsh::{BorshDeserialize, BorshSerialize};
use ivm_zkvm::{Risc0, Sp1, Zkvm};

const HIGH_CYCLE_LIMIT: u64 = 1_000_000_000;

/// Input parameters for the intensity test
#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct IntensityInput {
    /// Number of hash rounds to perform
    pub hash_rounds: u32,
}

/// Compares execution time between RISC0 and SP1 VMs for different workload intensities
pub fn compare_vms(risc0_elf: &[u8], sp1_elf: &[u8], intensities: &[u32]) {
    for &intensity in intensities {
        let input = IntensityInput { hash_rounds: intensity };
        let input_bytes = borsh::to_vec(&input).unwrap();

        // Test Risc0
        let start = std::time::Instant::now();
        let risc0_result = Risc0.execute(risc0_elf, &input_bytes, &[], HIGH_CYCLE_LIMIT).unwrap();
        let risc0_time = start.elapsed();

        // Test SP1
        let start = std::time::Instant::now();
        let sp1_result = Sp1.execute(sp1_elf, &input_bytes, &[], HIGH_CYCLE_LIMIT).unwrap();
        let sp1_time = start.elapsed();

        assert_eq!(risc0_result, sp1_result, "Full outputs don't match");

        let risc0_hash = &risc0_result[risc0_result.len() - 32..];
        let sp1_hash = &sp1_result[sp1_result.len() - 32..];
        assert_eq!(risc0_hash, sp1_hash, "Hash results don't match");

        println!("\nIntensity (hash rounds): {}", intensity);
        println!("Risc0 time: {:?}", risc0_time);
        println!("SP1 time: {:?}", sp1_time);
        println!(
            "Speed ratio (SP1/Risc0): {:.2}x",
            risc0_time.as_secs_f64() / sp1_time.as_secs_f64()
        );
    }
}
