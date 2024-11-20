use ivm_sp1_utils::get_program_id;
use once_cell::sync::Lazy;

pub const INTENSITY_TEST_ELF: &[u8] =
    include_bytes!("../../../target/sp1/intensity-test/intensity-test-sp1-guest");

static INTENSITY_TEST_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let program_id_path = "target/sp1/intensity-test/intensity-test-sp1-guest.vkey";
    get_program_id(program_id_path)
});

pub fn get_intensity_test_program_id() -> [u8; 32] {
    *INTENSITY_TEST_PROGRAM_ID
}
