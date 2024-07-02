use integration::{Clients, Integration};
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
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
async fn executor_works() {
    async fn test(mut clients: Clients) {
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
        let image_id = compute_image_id(&vapenation_elf).unwrap();

        let program_input = alloy_rlp::encode(3u64);

        let original_inputs = VerifiedInputs {
            program_verifying_key: image_id.as_bytes().to_vec(),
            program_input: program_input.clone(),
            max_cycles: 32 * 1000 * 1000,
        };
        let request = ExecuteRequest { program_elf: vapenation_elf, inputs: Some(original_inputs.clone()) };

        let ExecuteResponse { inputs, raw_output: _, zkvm_operator_address: _, zkvm_operator_signature: _ } =
            clients.executor.execute(request).await.unwrap().into_inner();
        let inputs = inputs.unwrap();

        assert_eq!(original_inputs, inputs);
    }

    Integration::run(test).await
}
