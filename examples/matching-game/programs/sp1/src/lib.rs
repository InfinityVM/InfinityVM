use once_cell::sync::Lazy;
use std::fs;

pub const MATCHING_GAME_ELF: &[u8] =
    include_bytes!("../program/riscv32im-succinct-zkvm-elf/matching-game-sp1-guest");

static MATCHING_GAME_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let vkey_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program/riscv32im-succinct-zkvm-elf/matching-game-sp1-guest.vkey"
    );
    let vk_bytes: Vec<u8> =
        bincode::deserialize_from(fs::File::open(vkey_path).expect("Failed to open vkey file"))
            .expect("Failed to deserialize verifying key");
    vk_bytes.try_into().expect("Invalid verifying key length")
});

pub fn get_matching_game_elf_id() -> [u8; 32] {
    *MATCHING_GAME_ELF_ID
}
