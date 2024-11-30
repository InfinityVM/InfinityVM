pub const MOCK_CONSUMER_ELF: &[u8] =
    include_bytes!("../../../target/sp1/mock-consumer/mock-consumer-sp1-guest");

pub const MOCK_CONSUMER_PROGRAM_ID: [u8; 32] =
    *include_bytes!("../../../target/sp1/mock-consumer/mock-consumer-sp1-guest.vkey");
