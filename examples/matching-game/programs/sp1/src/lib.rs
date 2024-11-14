use once_cell::sync::Lazy;
use sp1_sdk::{HashableKey, ProverClient};

pub const MATCHING_GAME_ELF: &[u8] = include_bytes!(
    "../program/elf-compilation/riscv32im-succinct-zkvm-elf/release/matching-game-sp1-guest"
);

static MATCHING_GAME_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let (_, vk) = ProverClient::new().setup(MATCHING_GAME_ELF);
    vk.hash_bytes().to_owned()
});

pub fn get_matching_game_elf_id() -> [u8; 32] {
    *MATCHING_GAME_ELF_ID
}
