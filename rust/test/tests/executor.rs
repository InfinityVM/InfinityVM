use alloy::{
    primitives::{keccak256, utils::eip191_hash_message, Address, Signature},
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use alloy_rlp::Decodable;
use alloy_sol_types::{sol, SolType};
use integration::{Clients, Integration};
use proto::{
    CreateElfRequest, CreateElfResponse, ExecuteRequest, ExecuteResponse, JobInputs, VmType,
};
use risc0_binfmt::compute_image_id;

use vapenation_core::{VapeNationArg, VapeNationMetadata};
use vapenation_methods::{VAPENATION_GUEST_ELF, VAPENATION_GUEST_ID, VAPENATION_GUEST_PATH};
use zkvm_executor::DEV_SECRET;

// WARNING: read the integration test readme to learn about common footguns while iterating
// on these integration tests.

const VAPENATION_ELF_PATH: &str =
    "../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest";

// TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
// const VAPENATION_ELF_SP1_PATH: &str =
//     "../programs/sp1/vapenation/program/elf/riscv32im-succinct-zkvm-elf";

fn expected_signer_address() -> Address {
    let signer = LocalSigner::<SigningKey>::from_slice(&DEV_SECRET).unwrap();
    signer.address()
}

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
type ResultWithMetadata = sol! {
    tuple(uint32,bytes32,uint64,bytes,bytes)
};

fn abi_encode_result_with_metadata(i: &JobInputs, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.program_input);
    ResultWithMetadata::abi_encode_params(&(
        i.job_id,
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}

#[test]
#[ignore]
fn invariants() {
    VAPENATION_GUEST_PATH
        .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest");
    VAPENATION_ELF_PATH
        .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest");

    let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
    let image_id = compute_image_id(&vapenation_elf).unwrap();

    assert_eq!(VAPENATION_GUEST_ELF, vapenation_elf);
    assert_eq!(&VAPENATION_GUEST_ID, image_id.as_words());
}

#[tokio::test]
#[ignore]
async fn executor_risc0_works() {
    async fn test(mut clients: Clients) {
        // Construct the request
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
        let image_id = compute_image_id(&vapenation_elf).unwrap().as_bytes().to_vec();
        let max_cycles = 32 * 1024 * 1024;
        let input = 2u64;
        let program_input = VapeNationArg::abi_encode(&input);

        let create_elf_request =
            CreateElfRequest { program_elf: vapenation_elf, vm_type: VmType::Risc0.into() };
        let CreateElfResponse { verifying_key } =
            clients.executor.create_elf(create_elf_request).await.unwrap().into_inner();
        assert_eq!(verifying_key, image_id);

        let original_inputs = JobInputs {
            job_id: 42069,
            program_verifying_key: image_id,
            program_input: program_input.clone(),
            max_cycles,
        };
        let request = ExecuteRequest { inputs: Some(original_inputs.clone()) };

        // Make a request and wait for the response

        let r = clients.executor.execute(request).await.unwrap().into_inner();
        let ExecuteResponse {
            inputs,
            raw_output,
            result_with_metadata,
            zkvm_operator_address,
            zkvm_operator_signature,
        } = r;

        // Verify address
        let address = {
            let address = String::from_utf8(zkvm_operator_address).unwrap();
            Address::parse_checksummed(address, None).unwrap()
        };
        assert_eq!(address, expected_signer_address());

        // Verify signature
        let sig = Signature::try_from(&zkvm_operator_signature[..]).unwrap();
        let signing_payload = abi_encode_result_with_metadata(&original_inputs, &raw_output);
        assert_eq!(result_with_metadata, signing_payload);

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

    Integration::run(test).await;
}

// TODO: https://github.com/Ethos-Works/InfinityVM/issues/120
// #[tokio::test]
// #[ignore]
// async fn executor_sp1_works() {
//     async fn test(mut clients: Clients) {
//         // Construct the request
//         let vapenation_elf = std::fs::read(VAPENATION_ELF_SP1_PATH).unwrap();
//         let client = ProverClient::new();

//         let (_, vk) = client.setup(vapenation_elf.as_slice());
//         let image_id = vk.hash_bytes().to_vec();
//         let max_cycles = 32 * 1024 * 1024;
//         let input = 2u64;
//         let program_input = VapeNationArg::abi_encode(&input);

//         let create_elf_request =
//             CreateElfRequest { program_elf: vapenation_elf, vm_type: VmType::Sp1.into() };
//         let CreateElfResponse { verifying_key } =
//             clients.executor.create_elf(create_elf_request).await.unwrap().into_inner();
//         assert_eq!(verifying_key, image_id);

//         let original_inputs = JobInputs {
//             job_id: 42069,
//             program_verifying_key: image_id,
//             program_input: program_input.clone(),
//             max_cycles,
//         };
//         let request = ExecuteRequest { inputs: Some(original_inputs.clone()) };

//         // Make a request and wait for the response

//         let r = clients.executor.execute(request).await.unwrap().into_inner();
//         let ExecuteResponse {
//             inputs,
//             raw_output,
//             result_with_metadata,
//             zkvm_operator_address,
//             zkvm_operator_signature,
//         } = r;

//         // // Verify address
//         let address = {
//             let address = String::from_utf8(zkvm_operator_address).unwrap();
//             Address::parse_checksummed(address, None).unwrap()
//         };
//         assert_eq!(address, expected_signer_address());

//         // Verify signature
//         let sig = Signature::decode_rlp_vrs(&mut &zkvm_operator_signature[..]).unwrap();
//         let signing_payload = abi_encode_result_with_metadata(&original_inputs, &raw_output);
//         assert_eq!(result_with_metadata, signing_payload);

//         let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
//         assert_eq!(recovered1, expected_signer_address());

//         // confirm we are hashing as expected
//         let hash = eip191_hash_message(&signing_payload);
//         let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
//         assert_eq!(recovered2, expected_signer_address());

//         // Verify input
//         let inputs = inputs.unwrap();
//         assert_eq!(original_inputs, inputs);

//         // Verify output
//         let metadata = VapeNationMetadata::decode(&mut &raw_output[..]).unwrap();
//         let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

//         assert_eq!(metadata.nation_id, 352380);
//         assert_eq!(metadata.points, 5106);
//         assert_eq!(metadata.phrase, phrase);
//     }

//     Integration::run(test).await
// }
