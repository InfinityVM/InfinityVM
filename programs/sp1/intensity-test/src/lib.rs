use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

pub const INTENSITY_TEST_SP1_GUEST_ELF: &[u8] =
    include_bytes!("../../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/intensity-test-sp1-guest");

static INTENSITY_TEST_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let (_, vk) = ProverClient::new().setup(INTENSITY_TEST_SP1_GUEST_ELF);
    vk.hash_bytes().to_owned()
});

pub fn get_intensity_test_elf_id() -> [u8; 32] {
    *INTENSITY_TEST_ELF_ID
}
