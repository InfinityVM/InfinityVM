use alloy_rlp::Decodable;
use alloy_sol_types::SolType;
use integration::{Clients, Integration};
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
use risc0_binfmt::compute_image_id;

use vapenation_core::{VapeNationArg, VapeNationMetadata};
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
        let max_cycles = 32 * 1024 * 1024;

        let input = 2u64;
        let program_input = VapeNationArg::abi_encode(&input);

        let original_inputs = VerifiedInputs {
            program_verifying_key: image_id.as_bytes().to_vec(),
            program_input: program_input.clone(),
            max_cycles,
        };
        let request =
            ExecuteRequest { program_elf: vapenation_elf, inputs: Some(original_inputs.clone()) };

        // TODO(zeke): verify signature
        let ExecuteResponse {
            inputs,
            raw_output,
            zkvm_operator_address: _,
            zkvm_operator_signature: _,
        } = clients.executor.execute(request).await.unwrap().into_inner();

        let inputs = inputs.unwrap();
        assert_eq!(original_inputs, inputs);

        let metadata = VapeNationMetadata::decode(&mut &raw_output[..]).unwrap();
        let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

        assert_eq!(metadata.nation_id, 352380);
        assert_eq!(metadata.points, 5106);
        assert_eq!(metadata.phrase, phrase);
    }

    Integration::run(test).await
}
