use integration::{Clients, Integration};
use proto::{ExecuteRequest, VerifiedInputs};
use risc0_binfmt::compute_image_id;

use vapenation_methods::{VAPENATION_GUEST_ELF, VAPENATION_GUEST_ID, VAPENATION_GUEST_PATH};

const VAPENATION_ELF_PATH: &str =
    "../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation_guest";

#[test]
#[ignore]
fn invariants() {
    VAPENATION_GUEST_PATH
        .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation_guest");
    VAPENATION_ELF_PATH
        .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation_guest");

    let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
    let image_id = compute_image_id(&vapenation_elf).unwrap();

    assert_eq!(VAPENATION_GUEST_ELF, vapenation_elf);
    assert_eq!(&VAPENATION_GUEST_ID, image_id.as_words());
}

#[tokio::test]
#[ignore]
async fn executor_fails_with_bad_inputs() {
    async fn test(mut clients: Clients) {
        let request = ExecuteRequest {
            program_elf: vec![],
            inputs: Some(VerifiedInputs {
                program_verifying_key: vec![],
                program_input: vec![],
                max_cycles: 32 * 1000,
            }),
        };

        let result = clients.executor.execute(request).await;

        assert!(result.is_err());
    }

    Integration::run(test).await
}
