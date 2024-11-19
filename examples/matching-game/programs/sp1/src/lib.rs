use once_cell::sync::Lazy;
use sp1_utils::get_program_id;

pub const MATCHING_GAME_ELF: &[u8] = include_bytes!("../program/elf/matching-game-sp1-guest");

static MATCHING_GAME_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    get_program_id(concat!(env!("CARGO_MANIFEST_DIR"), "/program/elf/matching-game-sp1-guest.vkey"))
});

pub fn get_matching_game_program_id() -> [u8; 32] {
    *MATCHING_GAME_PROGRAM_ID
}
