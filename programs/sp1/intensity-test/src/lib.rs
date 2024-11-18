use once_cell::sync::Lazy;
use sp1_utils::get_elf_id;
use std::path::PathBuf;

pub const INTENSITY_TEST_SP1_GUEST_ELF: &[u8] = include_bytes!(
    "../../../../target/elf-compilation/riscv32im-succinct-zkvm-elf/release/intensity-test-sp1-guest"
);

static INTENSITY_TEST_ELF_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let vkey_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../target")
        .join("elf-compilation/riscv32im-succinct-zkvm-elf/release/")
        .join("intensity-test-sp1-guest.vkey");
    get_elf_id(vkey_path.to_str().unwrap())
});

pub fn get_intensity_test_elf_id() -> [u8; 32] {
    *INTENSITY_TEST_ELF_ID
}
