use alloy::{
    network::EthereumWallet,
    primitives::{aliases::U256, keccak256, utils::eip191_hash_message, Address, Bytes, Signature},
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    signers::{local::PrivateKeySigner, Signer},
    sol,
    sol_types::{SolEvent, SolType, SolValue},
};
use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
use integration::{Args, Integration};
use mock_consumer_methods::{MOCK_CONSUMER_GUEST_ELF, MOCK_CONSUMER_GUEST_ID};
use proto::{
    GetResultRequest, Job, JobStatus, JobStatusType, SubmitJobRequest, SubmitProgramRequest, VmType,
};
use risc0_binfmt::compute_image_id;
use risc0_zkp::core::digest::Digest;
use test_utils::MOCK_CONTRACT_MAX_CYCLES;

type MockConsumerOut = sol!((Address, U256));

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
type ResultWithMetadata = sol! {
    tuple(uint32,bytes32,uint64,bytes,bytes)
};

/// The payload that gets signed by the user/app for an offchain job request.
///
/// tuple(Nonce,MaxCycles,Consumer,ProgramID,ProgramInput)
pub type OffchainJobRequest = sol! {
    tuple(uint64,uint64,address,bytes,bytes)
};

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed an offchain job. Also the result payload the job manager contract expects.
///
/// tuple(ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
pub type OffchainResultWithMetadata = sol! {
    tuple(bytes32,uint64,bytes,bytes)
};

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(i: &Job, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.input);

    ResultWithMetadata::abi_encode_params(&(
        i.id.unwrap(),
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}

/// Returns an ABI-encoded offchain job request. This ABI-encoded response will be
/// signed by the entity sending the job request (user, app, authorized third-party, etc.).
pub fn abi_encode_offchain_job_request(job: Job) -> Vec<u8> {
    let nonce = job.nonce.unwrap();

    // Extract the last 20 bytes to get the actual Ethereum address
    let contract_address: [u8; 20] =
        job.contract_address[job.contract_address.len() - 20..].try_into().unwrap();

    OffchainJobRequest::abi_encode_params(&(
        nonce,
        job.max_cycles,
        contract_address,
        job.program_verifying_key,
        job.input,
    ))
}

/// Returns an ABI-encoded offchain result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_offchain_result_with_metadata(i: &Job, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.input);

    OffchainResultWithMetadata::abi_encode_params(&(
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}

fn mock_consumer_program_id() -> Digest {
    compute_image_id(MOCK_CONSUMER_GUEST_ELF).unwrap()
}

#[test]
#[ignore]
fn invariants() {
    let image_id = mock_consumer_program_id();

    assert_eq!(&MOCK_CONSUMER_GUEST_ID, image_id.as_words());
}

#[tokio::test]
#[ignore]
async fn web2_job_submission_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let chain_id = anvil.anvil.chain_id();
        let program_id = mock_consumer_program_id().as_bytes().to_vec();
        let mock_user_address = Address::repeat_byte(69);

        let random_user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();
        let random_user_wallet = EthereumWallet::from(random_user.clone());

        // Seed coprocessor-node with ELF
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
        assert_eq!(
            submit_program_response.program_verifying_key,
            program_id // Digest::new(MOCK_CONSUMER_GUEST_ID).as_bytes()
        );

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(random_user_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(anvil.mock_consumer, &consumer_provider);

        // Submit job to coproc node
        let mut job = Job {
            id: None,
            nonce: Some(1),
            max_cycles: MOCK_CONTRACT_MAX_CYCLES,
            contract_address: anvil.mock_consumer.abi_encode(),
            program_verifying_key: program_id.clone(),
            input: mock_user_address.abi_encode(),
            request_signature: vec![], // TODO
            result: vec![],
            zkvm_operator_address: vec![],
            zkvm_operator_signature: vec![],
            status: Some(JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            }),
        };

        // Add signature from user on job request
        let job_request_payload = abi_encode_offchain_job_request(job.clone());
        let request_signature = random_user.sign_message(&job_request_payload).await.unwrap();
        job.request_signature = request_signature.as_bytes().to_vec();

        let submit_job_request = SubmitJobRequest { job: Some(job.clone()) };
        let submit_job_response =
            args.coprocessor_node.submit_job(submit_job_request).await.unwrap().into_inner();
        assert_eq!(submit_job_response.nonce, 1);
        assert_eq!(submit_job_response.contract_address, anvil.mock_consumer.abi_encode());

        // wait for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // This job ID will be generated by the contracts, and will be 1 here since
        // it's the first job submitted to the contracts.
        let job_id = 1;
        let get_result_request = GetResultRequest { job_id };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let Job { input, result, zkvm_operator_address, zkvm_operator_signature, status, .. } =
            get_result_response.job.unwrap();

        // Verify the job execution result
        let done_status: i32 = JobStatusType::Done.into();
        assert_eq!(status.unwrap().status, done_status);

        // Verify address
        let address = {
            let address = String::from_utf8(zkvm_operator_address).unwrap();
            Address::parse_checksummed(address, Some(chain_id)).unwrap()
        };
        assert_eq!(address, anvil.coprocessor_operator.address());

        // Verify signature and message format
        let sig = Signature::try_from(&zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output =
            OffchainResultWithMetadata::abi_decode_params(&result, false).unwrap();
        let raw_output = abi_decoded_output.3;
        let signing_payload = abi_encode_offchain_result_with_metadata(&job, &raw_output);
        assert_eq!(result, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), input);

        // Verify output from gRPC get_job endpoint
        let (out_address, out_balance) = MockConsumerOut::abi_decode(&raw_output, true).unwrap();

        let expected_balance = U256::from(21438208680330350173532883594u128);
        assert_eq!(out_address, mock_user_address);
        assert_eq!(out_balance, expected_balance);

        // Verify output on chain
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();
        let completed =
            logs[0].log_decode::<IJobManager::JobCompleted>().unwrap().data().to_owned();
        assert_eq!(completed.result, raw_output);
        assert_eq!(completed.jobID, 1);

        let get_balance_call = consumer_contract.getBalance(mock_user_address);
        let MockConsumer::getBalanceReturn { _0: balance } = get_balance_call.call().await.unwrap();
        assert_eq!(balance, expected_balance);
    }
    Integration::run(test).await;
}

#[ignore]
#[tokio::test]
async fn event_job_created_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let chain_id = anvil.anvil.chain_id();
        let program_id = mock_consumer_program_id().as_bytes().to_vec();
        let mock_user_address = Address::repeat_byte(69);

        let random_user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();
        let random_user_wallet = EthereumWallet::from(random_user);

        // Seed coprocessor-node with ELF
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
        assert_eq!(submit_program_response.program_verifying_key, program_id);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(random_user_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(anvil.mock_consumer, &consumer_provider);

        // Make onchain job request
        let create_job_call = consumer_contract
            .requestBalance(Bytes::copy_from_slice(&program_id), mock_user_address);
        let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
        // Confirm the call worked as expected
        let log = receipt.inner.as_receipt().unwrap().logs[0]
            .log_decode::<IJobManager::JobCreated>()
            .unwrap();
        let job_id = log.data().jobID;
        assert_eq!(job_id, 1);

        // Wait for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let get_result_request = GetResultRequest { job_id };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let job = get_result_response.job.unwrap();

        // Verify the job execution result
        let done_status = Some(JobStatus {
            status: JobStatusType::Done as i32,
            failure_reason: None,
            retries: 0,
        });
        assert_eq!(job.status, done_status);

        // Verify address
        let address = {
            let address = String::from_utf8(job.zkvm_operator_address.clone()).unwrap();
            Address::parse_checksummed(address, Some(chain_id)).unwrap()
        };
        assert_eq!(address, anvil.coprocessor_operator.address());

        // Verify signature and message format
        let sig = Signature::try_from(&job.zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output = ResultWithMetadata::abi_decode_params(&job.result, false).unwrap();
        let raw_output = abi_decoded_output.4;
        let signing_payload = abi_encode_result_with_metadata(&job, &raw_output);
        assert_eq!(job.result, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), job.input);

        // Verify output from gRPC get_job endpoint
        let (out_address, out_balance) = MockConsumerOut::abi_decode(&raw_output, true).unwrap();

        let expected_balance = U256::from(21438208680330350173532883594u128);
        assert_eq!(out_address, mock_user_address);
        assert_eq!(out_balance, expected_balance);

        // Verify output on chain
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();
        let completed =
            logs[0].log_decode::<IJobManager::JobCompleted>().unwrap().data().to_owned();
        assert_eq!(completed.result, raw_output);
        assert_eq!(completed.jobID, 1);

        let get_balance_call = consumer_contract.getBalance(mock_user_address);
        let MockConsumer::getBalanceReturn { _0: balance } = get_balance_call.call().await.unwrap();
        assert_eq!(balance, expected_balance);
    }
    Integration::run(test).await;
}
