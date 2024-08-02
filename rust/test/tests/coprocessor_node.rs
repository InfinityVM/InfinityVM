// use alloy::{
//     primitives::{keccak256, utils::eip191_hash_message, Address, Signature},
//     rlp::Decodable,
//     signers::{k256::ecdsa::SigningKey, local::LocalSigner},
//     sol,
//     sol_types::SolType,
// };
// use integration::{Args, Integration};
// use proto::{GetResultRequest, Job, JobStatus, SubmitJobRequest, SubmitProgramRequest, VmType};

// use risc0_binfmt::compute_image_id;
// use risc0_zkp::core::digest::Digest;
// use vapenation_core::{VapeNationArg, VapeNationMetadata};
// use vapenation_methods::{VAPENATION_GUEST_ELF, VAPENATION_GUEST_ID, VAPENATION_GUEST_PATH};
// use zkvm_executor::DEV_SECRET;

// // WARNING: read the integration test readme to learn about common footguns while iterating
// // on these integration tests.

// const VAPENATION_ELF_PATH: &str =
//     "../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest";

// // TODO: Add back in SP1 [ref: https://github.com/Ethos-Works/InfinityVM/issues/120]
// // const VAPENATION_ELF_SP1_PATH: &str =
// //     "../programs/sp1/vapenation/program/elf/riscv32im-succinct-zkvm-elf";

// fn expected_signer_address() -> Address {
//     let signer = LocalSigner::<SigningKey>::from_slice(&DEV_SECRET).unwrap();
//     signer.address()
// }

// /// The payload that gets signed to signify that the zkvm executor has faithfully
// /// executed the job. Also the result payload the job manager contract expects.
// ///
// /// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
// type ResultWithMetadata = sol! {
//     tuple(uint32,bytes32,uint64,bytes,bytes)
// };

// /// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
// /// signed by the operator.
// pub fn abi_encode_result_with_metadata(i: &Job, raw_output: &[u8]) -> Vec<u8> {
//     let program_input_hash = keccak256(&i.input);

//     ResultWithMetadata::abi_encode_params(&(
//         i.id,
//         program_input_hash,
//         i.max_cycles,
//         &i.program_verifying_key,
//         raw_output,
//     ))
// }

// #[test]
// #[ignore]
// fn invariants() {
//     VAPENATION_GUEST_PATH
//         .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest");
//     VAPENATION_ELF_PATH
//         .contains("/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest");

//     let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
//     let image_id = compute_image_id(&vapenation_elf).unwrap();

//     assert_eq!(VAPENATION_GUEST_ELF, vapenation_elf);
//     assert_eq!(&VAPENATION_GUEST_ID, image_id.as_words());
// }

// // #[tokio::test]
// // #[ignore]
// // Tests all gRPC endpoints of the coprocessor node: submit_program, submit_job, and
// `get_result`. async fn coprocessor_node_risc0_works() {
//     async fn test(mut args: Args) {
//         let chain_id = args.anvil.anvil.chain_id();

//         let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
//         let submit_program_request = SubmitProgramRequest {
//             program_elf: vapenation_elf.clone(),
//             vm_type: VmType::Risc0.into(),
//         };
//         let submit_program_response = args
//             .coprocessor_node
//             .submit_program(submit_program_request)
//             .await
//             .unwrap()
//             .into_inner();
//         let program_id = submit_program_response.program_verifying_key;
//         let input = 2u64;
//         let original_input = VapeNationArg::abi_encode(&input);

//         // Check the program ID returned by the coprocessor node is correct
//         assert_eq!(program_id, Digest::new(VAPENATION_GUEST_ID).as_bytes().to_vec());

//         // Submit a job
//         let job_id = 1;
//         let job = Job {
//             id: job_id,
//             max_cycles: 32 * 1024 * 1024,
//             contract_address: "0x0000000000000000000000000000000000000000".into(),
//             program_verifying_key: program_id,
//             input: original_input.clone(),
//             result: vec![],
//             zkvm_operator_address: vec![],
//             zkvm_operator_signature: vec![],
//             status: JobStatus::Pending.into(),
//         };
//         let submit_job_request = SubmitJobRequest { job: Some(job.clone()) };
//         let submit_job_response =
//             args.coprocessor_node.submit_job(submit_job_request).await.unwrap().into_inner();
//         assert_eq!(submit_job_response.job_id, job_id);

//         // Wait a bit for the job to be processed
//         tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

//         // Get the job result
//         let get_result_request = GetResultRequest { job_id };
//         let get_result_response =
//             args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
//         let Job {
//             id: _,
//             max_cycles: _,
//             contract_address: _,
//             program_verifying_key: _,
//             input,
//             result,
//             zkvm_operator_address,
//             zkvm_operator_signature,
//             status,
//         } = get_result_response.job.unwrap();

//         // Verify the job execution result
//         let done_status: i32 = JobStatus::Done.into();
//         assert_eq!(status, done_status);

//         // Verify address
//         let address = {
//             let address = String::from_utf8(zkvm_operator_address).unwrap();
//             Address::parse_checksummed(address, Some(chain_id)).unwrap()
//         };
//         assert_eq!(address, expected_signer_address());

//         // Verify signature
//         let sig = Signature::try_from(&zkvm_operator_signature[..]).unwrap();
//         let abi_decoded_output = ResultWithMetadata::abi_decode_params(&result, false).unwrap();
//         let raw_output = abi_decoded_output.4;
//         let signing_payload = abi_encode_result_with_metadata(&job, &raw_output);
//         assert_eq!(result, signing_payload);

//         let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
//         assert_eq!(recovered1, expected_signer_address());

//         // confirm we are hashing as expected
//         let hash = eip191_hash_message(&signing_payload);
//         let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
//         assert_eq!(recovered2, expected_signer_address());

//         // Verify input
//         assert_eq!(original_input, input);

//         // Verify output
//         let metadata = VapeNationMetadata::decode(&mut &raw_output[..]).unwrap();
//         let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

//         assert_eq!(metadata.nation_id, 352380);
//         assert_eq!(metadata.points, 5106);
//         assert_eq!(metadata.phrase, phrase);
//     }

//     Integration::run(test).await;
// }

// // TODO: Add back in SP1 [ref: https://github.com/Ethos-Works/InfinityVM/issues/120]
