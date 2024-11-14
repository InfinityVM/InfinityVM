use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

pub const CLOB_ELF: &[u8] =
    include_bytes!("../program/elf-compilation/riscv32im-succinct-zkvm-elf/release/clob-sp1-guest");

static CLOB_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let (_, vk) = ProverClient::new().setup(CLOB_ELF);
    vk.hash_bytes().to_owned()
});

pub fn get_clob_elf_id() -> [u8; 32] {
    *CLOB_ELF_ID
}
