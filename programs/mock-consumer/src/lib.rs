use once_cell::sync::Lazy;
use sp1_utils::get_program_id;
use std::path::PathBuf;

pub const MOCK_CONSUMER_ELF: &[u8] = include_bytes!("../program/elf/mock-consumer-sp1-guest");

static MOCK_CONSUMER_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let program_id_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("program/elf")
        .join("mock-consumer-sp1-guest.vkey");
    get_program_id(program_id_path.to_str().unwrap())
});

pub fn get_mock_consumer_program_id() -> [u8; 32] {
    *MOCK_CONSUMER_PROGRAM_ID
}
