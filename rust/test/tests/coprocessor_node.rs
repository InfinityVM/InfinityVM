use alloy::{
    primitives::{keccak256, utils::eip191_hash_message, Address, Signature},
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
};
use alloy_rlp::Decodable;
use alloy_sol_types::{sol, SolType};
use integration::{Clients, Integration};
use proto::{
    GetResultRequest, Job, JobStatus, SubmitJobRequest, SubmitProgramRequest, VmType,
    JobInputs,
};

use vapenation_core::{VapeNationArg, VapeNationMetadata};
use zkvm_executor::DEV_SECRET;

// WARNING: read the integration test readme to learn about common footguns while iterating
// on these integration tests.

const VAPENATION_ELF_PATH: &str =
    "../target/riscv-guest/riscv32im-risc0-zkvm-elf/release/vapenation-guest";

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

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(i: &Job, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.input);

    ResultWithMetadata::abi_encode_params(&(
        i.id,
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}
    
#[tokio::test]
#[ignore]
async fn coprocessor_node_risc0_works() {
    async fn test(mut clients: Clients) {
        let vapenation_elf = std::fs::read(VAPENATION_ELF_PATH).unwrap();
        let submit_program_request = SubmitProgramRequest {
            program_elf: vapenation_elf.clone(),
            vm_type: VmType::Risc0.into(),
        };
        let submit_program_response = clients.coprocessor_node
            .submit_program(submit_program_request)
            .await.unwrap()
            .into_inner();
        let verifying_key = submit_program_response.program_verifying_key;
        let input = 2u64;
        let original_input = VapeNationArg::abi_encode(&input);
    
        // Submit a job
        let job_id = 1;
        let job = Job {
            id: job_id,
            max_cycles: 32 * 1024 * 1024,
            contract_address: "0x0000000000000000000000000000000000000000".into(),
            program_verifying_key: verifying_key.clone(),
            input: original_input.clone(),
            result: vec![],
            zkvm_operator_address: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus::Pending.into(),
        };
        let submit_job_request = SubmitJobRequest { job: Some(job.clone()) };
        let submit_job_response = clients.coprocessor_node.submit_job(submit_job_request).await.unwrap().into_inner();
        assert_eq!(submit_job_response.job_id, job_id);
    
        // Wait a bit for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
        // Get the job result
        let get_result_request = GetResultRequest { job_id };
        let get_result_response = clients.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let Job { 
            id,
            max_cycles,
            contract_address,
            program_verifying_key,
            input,
            result,
            zkvm_operator_address,
            zkvm_operator_signature,
            status,
        }  = get_result_response.job.unwrap();
        
        // Verify the job execution result
        let done_status: i32 = JobStatus::Done.into();
        assert_eq!(status, done_status);

        // Verify address
        let address = {
            let address = String::from_utf8(zkvm_operator_address).unwrap();
            Address::parse_checksummed(address, Some(1)).unwrap()
        };
        assert_eq!(address, expected_signer_address());
        
        // Verify signature
        let sig = Signature::decode_rlp_vrs(&mut &zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output = ResultWithMetadata::abi_decode_params(&result, false).unwrap();
        let raw_output = abi_decoded_output.4;   
        let signing_payload = abi_encode_result_with_metadata(&job, &raw_output);
        assert_eq!(result, signing_payload);
        
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, expected_signer_address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, expected_signer_address());
        
        // Verify input
        assert_eq!(original_input, input);
        
        // Verify output
        let metadata = VapeNationMetadata::decode(&mut &raw_output[..]).unwrap();
        let phrase = (0..2).map(|_| "NeverForget420".to_string()).collect::<Vec<_>>().join(" ");

        assert_eq!(metadata.nation_id, 352380);
        assert_eq!(metadata.points, 5106);
        assert_eq!(metadata.phrase, phrase);
    }

    Integration::run(test).await;
}
