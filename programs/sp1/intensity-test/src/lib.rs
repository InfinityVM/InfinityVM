use once_cell::sync::Lazy;
use std::{fs, path::PathBuf};

pub const INTENSITY_TEST_SP1_GUEST_ELF: &[u8] = include_bytes!(
    "../../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/intensity-test-sp1-guest"
);

static INTENSITY_TEST_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let vkey_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target")
        .join("elf-compilation/riscv32im-succinct-zkvm-elf/release/")
        .join("intensity-test-sp1-guest.vkey");

    let vk_bytes: Vec<u8> =
        bincode::deserialize_from(fs::File::open(&vkey_path).expect("Failed to open vkey file"))
            .expect("Failed to deserialize verifying key");
    vk_bytes.try_into().expect("Invalid verifying key length")
});

pub fn get_intensity_test_elf_id() -> [u8; 32] {
    *INTENSITY_TEST_ELF_ID
}