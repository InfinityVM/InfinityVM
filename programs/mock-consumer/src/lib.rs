use ivm_sp1_utils::get_program_id;
use once_cell::sync::Lazy;

pub const MOCK_CONSUMER_ELF: &[u8] =
    include_bytes!("../../../target/sp1/mock-consumer/mock-consumer-sp1-guest");

static MOCK_CONSUMER_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    let program_id_path = "target/sp1/mock-consumer/mock-consumer-sp1-guest.vkey";
    get_program_id(program_id_path)
});

pub fn get_mock_consumer_program_id() -> [u8; 32] {
    *MOCK_CONSUMER_PROGRAM_ID
}
