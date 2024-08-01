use alloy::primitives::aliases::U256;
use alloy::{
    network::EthereumWallet,
    primitives::{keccak256, utils::eip191_hash_message, Address, Signature},
    providers::ProviderBuilder,
    rlp::Decodable,
    signers::local::PrivateKeySigner,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
    sol,
    sol_types::SolType,
    sol_types::SolValue,
};
use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
use integration::Args;
use integration::Integration;
use mock_consumer_methods::{
    MOCK_CONSUMER_GUEST_ELF, MOCK_CONSUMER_GUEST_ID, MOCK_CONSUMER_GUEST_PATH,
};
use proto::{GetResultRequest, Job, JobStatus, SubmitJobRequest, SubmitProgramRequest, VmType};
use risc0_binfmt::compute_image_id;
use risc0_zkp::core::digest::Digest;
use test_utils::MOCK_CONTRACT_MAX_CYCLES;

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
async fn coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let chain_id = anvil.anvil.chain_id();
        let program_id = Digest::new(MOCK_CONSUMER_GUEST_ID).as_bytes().to_vec();
        let mock_user_address = Address::repeat_byte(42);

        let user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();
        let user_wallet = EthereumWallet::from(user);

        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_GUEST_ELF.to_vec(),
            vm_type: VmType::Risc0.into(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(program_id, Digest::new(MOCK_CONSUMER_GUEST_ID).as_bytes().to_vec());

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(user_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(anvil.mock_consumer, &consumer_provider);

        // Make onchain job request
        let create_job_call =
            consumer_contract.requestBalance(program_id.clone().into(), mock_user_address);

        let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
        let log = receipt.inner.as_receipt().unwrap().logs[0]
            .log_decode::<IJobManager::JobCreated>()
            .unwrap();
        let job_id = log.data().jobID;

        // Submit job to coproc nodes
        let job = Job {
            id: job_id,
            max_cycles: MOCK_CONTRACT_MAX_CYCLES,
            contract_address: anvil.mock_consumer.abi_encode(),
            program_verifying_key: program_id.clone(),
            input: mock_user_address.abi_encode(),
            result: vec![],
            zkvm_operator_address: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus::Pending.into(),
        };
        let submit_job_request = SubmitJobRequest { job: Some(job.clone()) };
        let submit_job_response =
            args.coprocessor_node.submit_job(submit_job_request).await.unwrap().into_inner();
        assert_eq!(submit_job_response.job_id, job_id);

        // wait for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let get_result_request = GetResultRequest { job_id };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let Job { input, result, zkvm_operator_address, zkvm_operator_signature, status, .. } =
            get_result_response.job.unwrap();

        // Verify the job execution result
        let done_status: i32 = JobStatus::Done.into();
        assert_eq!(status, done_status);

        // Verify address
        let address = {
            let address = String::from_utf8(zkvm_operator_address).unwrap();
            Address::parse_checksummed(address, Some(chain_id)).unwrap()
        };
        assert_eq!(address, anvil.coprocessor_operator.address());

        let sig = Signature::try_from(&zkvm_operator_signature[..]).unwrap();

        let abi_decoded_output = ResultWithMetadata::abi_decode_params(&result, false).unwrap();
        let raw_output = abi_decoded_output.4;
        let signing_payload = abi_encode_result_with_metadata(&job, &raw_output);
        assert_eq!(result, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(address, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), input);

        // Verify output
        let balance = U256::try_from_be_slice(&raw_output[..]);
        dbg!(balance);
    }
    Integration::run(test).await;
}
