//! VM comparison benchmark binary
const INTENSITY_TEST_RISC0_GUEST_ELF: &[u8] = include_bytes!(
    "../../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/intensity-test-guest"
);
const INTENSITY_TEST_SP1_GUEST_ELF: &[u8] = include_bytes!("../../../elf/intensity-test-sp1-guest");

fn main() {
    println!("Running VM speed comparison benchmark...\n");

    let intensities = [1000, 5000, 10000, 50000];

    println!("Testing with intensities (hash rounds): {:?}\n", intensities);
    println!("Higher ratios mean SP1 is faster relative to RISC0\n");

    vm_comparison::compare_vms(
        INTENSITY_TEST_RISC0_GUEST_ELF,
        INTENSITY_TEST_SP1_GUEST_ELF,
        &intensities,
    );
}
