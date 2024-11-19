use once_cell::sync::Lazy;
use sp1_utils::get_program_id;

pub const CLOB_ELF: &[u8] = include_bytes!("../program/elf/clob-sp1-guest");

static CLOB_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    get_program_id(concat!(env!("CARGO_MANIFEST_DIR"), "/program/elf/clob-sp1-guest.vkey"))
});

pub fn get_clob_program_id() -> [u8; 32] {
    *CLOB_PROGRAM_ID
}
