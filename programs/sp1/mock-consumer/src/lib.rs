use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const MOCK_CONSUMER_SP1_GUEST_ELF: &[u8] =
    include_bytes!("../../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/mock-consumer-sp1-guest");

static MOCK_CONSUMER_SP1_GUEST_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let (_, vk) = ProverClient::new().setup(MOCK_CONSUMER_SP1_GUEST_ELF);
    vk.hash_bytes().to_owned()
});

pub fn get_mock_consumer_sp1_guest_elf_id() -> [u8; 32] {
    *MOCK_CONSUMER_SP1_GUEST_ELF_ID
}
