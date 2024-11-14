//! VM speed benchmark binary

fn main() {
    println!("Running VM speed benchmark...\n");

    let sp1_elf = std::fs::read(
        "../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/intensity-test-sp1-guest",
    )
    .unwrap();

    let intensities = [1000, 5000, 10000, 50000];
    println!("Testing with intensities (hash rounds): {:?}\n", intensities);

    vm_comparison::compare_vms(&sp1_elf, &intensities);
}
