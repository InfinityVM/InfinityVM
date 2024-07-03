use alloy::{
    primitives::{utils::eip191_hash_message, Address, Signature},
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use alloy_rlp::Decodable;
use alloy_sol_types::SolType;
use integration::{Clients, Integration};
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
use risc0_binfmt::compute_image_id;

use executor::DEV_SECRET;
use vapenation_core::{VapeNationArg, VapeNationMetadata};
use vapenation_methods::{VAPENATION_GUEST_ELF, VAPENATION_GUEST_ID, VAPENATION_GUEST_PATH};

// WARNING: read the integration test readme to learn about common footguns while iterating
// on these integration tests.

const VAPENATION_ELF_PATH: &str =
    "../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation_guest";

fn expected_signer_address() -> Address {
    let signer = LocalSigner::<SigningKey>::from_slice(&DEV_SECRET).unwrap();
    signer.address()
}

// We copy and paste this instead of importing so we can detect regressions in the core impl.
fn result_signing_payload(i: &VerifiedInputs, raw_output: &[u8]) -> Vec<u8> {
    i.program_verifying_key
        .iter()
        .chain(i.program_input.iter())
        .chain(i.max_cycles.to_be_bytes().iter())
        .chain(raw_output)
        .copied()
        .collect()
}

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
        // Construct the request
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

        // Make a request and wait for the response
        let ExecuteResponse { inputs, raw_output, zkvm_operator_address, zkvm_operator_signature } =
            clients.executor.execute(request).await.unwrap().into_inner();

        // Verify address
        let address = {
            let address = String::from_utf8(zkvm_operator_address).unwrap();
            Address::parse_checksummed(address, None).unwrap()
        };
        assert_eq!(address, expected_signer_address());

        // Verify signature
        // Note: alternatively we could use decode_rlp_vrs if we did not encode with a header
        let sig = Signature::decode(&mut &zkvm_operator_signature[..]).unwrap();
        let signing_payload = result_signing_payload(&original_inputs, &raw_output);

        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, expected_signer_address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, expected_signer_address());

        // Verify input
        let inputs = inputs.unwrap();
        assert_eq!(original_inputs, inputs);

        // Verify output
        let metadata = VapeNationMetadata::decode(&mut &raw_output[..]).unwrap();
        let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

        assert_eq!(metadata.nation_id, 352380);
        assert_eq!(metadata.points, 5106);
        assert_eq!(metadata.phrase, phrase);
    }

    Integration::run(test).await
}
