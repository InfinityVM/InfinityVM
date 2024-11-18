use once_cell::sync::Lazy;
use std::fs;

pub const CLOB_ELF: &[u8] = include_bytes!("../program/elf/clob-sp1-guest");

static CLOB_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let vkey_path = concat!(env!("CARGO_MANIFEST_DIR"), "/program/elf/clob-sp1-guest.vkey");
    let vk_bytes: Vec<u8> =
        bincode::deserialize_from(fs::File::open(vkey_path).expect("Failed to open vkey file"))
            .expect("Failed to deserialize verifying key");
    vk_bytes.try_into().expect("Invalid verifying key length")
});

pub fn get_clob_elf_id() -> [u8; 32] {
    *CLOB_ELF_ID
}
