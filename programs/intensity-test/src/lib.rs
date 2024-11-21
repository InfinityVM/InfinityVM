pub const INTENSITY_TEST_ELF: &[u8] =
    include_bytes!("../../../target/sp1/intensity-test/intensity-test-sp1-guest");

pub const INTENSITY_TEST_PROGRAM_ID: [u8; 32] =
    *include_bytes!("../../../target/sp1/intensity-test/intensity-test-sp1-guest.vkey");
