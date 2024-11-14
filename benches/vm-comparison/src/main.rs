//! VM comparison benchmark binary

fn main() {
    println!("Running VM speed comparison benchmark...\n");

    let risc0_elf = std::fs::read(
        "../../../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/intensity-test-guest",
    )
    .unwrap();
    let sp1_elf = std::fs::read(
        "../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/intensity-test-sp1-guest",
    )
    .unwrap();

    let intensities = [1000, 5000, 10000, 50000];
    println!("Testing with intensities (hash rounds): {:?}\n", intensities);
    println!("Higher ratios mean SP1 is faster relative to RISC0\n");

    vm_comparison::compare_vms(&risc0_elf, &sp1_elf, &intensities);
}
