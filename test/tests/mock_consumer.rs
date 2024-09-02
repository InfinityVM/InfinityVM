use abi::{abi_encode_offchain_job_request, JobParams};
use alloy::{
    network::EthereumWallet,
    primitives::{
        aliases::U256, utils::eip191_hash_message, Address, Bytes, FixedBytes, Signature,
    },
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    signers::{local::PrivateKeySigner, Signer},
    sol,
    sol_types::{SolEvent, SolType, SolValue},
};
use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
use db::tables::{get_job_id, Job, RequestType};
use e2e::{Args, E2E};
use mock_consumer_methods::{MOCK_CONSUMER_GUEST_ELF, MOCK_CONSUMER_GUEST_ID};
use proto::{
    GetResultRequest, JobStatus, JobStatusType, SubmitProgramRequest, SubmitStatefulJobRequest,
    VmType,
};
use risc0_binfmt::compute_image_id;
use risc0_zkp::core::digest::Digest;
use test_utils::MOCK_CONTRACT_MAX_CYCLES;
use zkvm_executor::service::{abi_encode_result_with_metadata, ResultWithMetadata};

type MockConsumerOut = sol!((Address, U256));

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
        let mock = args.mock_consumer.unwrap();
        let anvil = args.anvil;
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
        assert_eq!(submit_program_response.program_id, program_id);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(random_user_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock.mock_consumer, &consumer_provider);

        // Submit job to coproc node
        let nonce = 1;
        let job_id = get_job_id(nonce, mock.mock_consumer);
        let mut job = Job {
            id: job_id,
            nonce,
            max_cycles: MOCK_CONTRACT_MAX_CYCLES,
            consumer_address: mock.mock_consumer.abi_encode_packed(),
            program_id: program_id.clone(),
            input: mock_user_address.abi_encode(),
            program_state: vec![],
            // signature added to this job below
            request_type: RequestType::Offchain(vec![]),
            result_with_metadata: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
        };

        // Add signature from user on job request
        let job_params = JobParams {
            nonce: job.nonce,
            max_cycles: job.max_cycles,
            consumer_address: job.consumer_address.clone().try_into().unwrap(),
            program_input: job.input.clone(),
            program_id: job.program_id.clone(),
        };
        let job_request_payload = abi_encode_offchain_job_request(job_params);
        let request_signature = random_user.sign_message(&job_request_payload).await.unwrap();
        job.request_type = RequestType::Offchain(request_signature.as_bytes().to_vec());

        let job_request = SubmitStatefulJobRequest {
            request: job_request_payload,
            signature: request_signature.into(),
            program_state: vec![],
        };
        let submit_stateful_job_response =
            args.coprocessor_node.submit_stateful_job(job_request).await.unwrap().into_inner();
        assert_eq!(submit_stateful_job_response.job_id, job_id);

        // wait for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let job_result = get_result_response.job_result.unwrap();

        // Verify the job execution result
        let done_status: i32 = JobStatusType::Done.into();
        assert_eq!(job_result.status.unwrap().status, done_status);

        // Verify signature and message format
        let sig = Signature::try_from(&job_result.zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output =
            ResultWithMetadata::abi_decode_params(&job_result.result_with_metadata, false).unwrap();

        let raw_output = abi_decoded_output.4;
        let signing_payload = abi_encode_result_with_metadata(
            job_id,
            &job_result.input,
            job_result.max_cycles,
            &job_result.program_id,
            &raw_output,
        )
        .unwrap();
        assert_eq!(job_result.result_with_metadata, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), job_result.input);

        // Verify output from gRPC get_job endpoint
        let (out_address, out_balance) = MockConsumerOut::abi_decode(&raw_output, true).unwrap();

        let expected_balance = U256::from(21438208680330350173532883594u128);
        assert_eq!(out_address, mock_user_address);
        assert_eq!(out_balance, expected_balance);

        // Verify output from onchain event
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();
        let completed =
            logs[0].log_decode::<IJobManager::JobCompleted>().unwrap().data().to_owned();
        assert_eq!(completed.result, raw_output);
        assert_eq!(completed.jobID, FixedBytes(job_id));

        // Verify balance onchain
        let get_balance_call = consumer_contract.getBalance(mock_user_address);
        let MockConsumer::getBalanceReturn { _0: balance } = get_balance_call.call().await.unwrap();
        assert_eq!(balance, expected_balance);

        // Verify inputs onchain
        let get_inputs_call = consumer_contract.getProgramInputsForJob(FixedBytes(job_id));
        let MockConsumer::getProgramInputsForJobReturn { _0: inputs } =
            get_inputs_call.call().await.unwrap();
        assert_eq!(Address::abi_encode(&mock_user_address), inputs);

        // Verify nonce onchain
        let get_next_nonce_call = consumer_contract.getNextNonce();
        let MockConsumer::getNextNonceReturn { _0: nonce } =
            get_next_nonce_call.call().await.unwrap();
        assert_eq!(nonce, 2);
    }
    E2E::new().mock_consumer().run(test).await;
}

#[ignore]
#[tokio::test]
async fn event_job_created_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let mock = args.mock_consumer.unwrap();
        let anvil = args.anvil;
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
        assert_eq!(submit_program_response.program_id, program_id);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(random_user_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock.mock_consumer, &consumer_provider);

        // Make onchain job request
        let create_job_call = consumer_contract
            .requestBalance(Bytes::copy_from_slice(&program_id), mock_user_address);
        let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
        // Confirm the call worked as expected
        let log = receipt.inner.as_receipt().unwrap().logs[0]
            .log_decode::<IJobManager::JobCreated>()
            .unwrap();
        let job_id = log.data().jobID;
        let expected_job_id = get_job_id(1, mock.mock_consumer);
        assert_eq!(job_id, expected_job_id);

        // Wait for the job to be processed
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let job_result = get_result_response.job_result.unwrap();

        // Verify the job execution result
        let done_status = JobStatusType::Done as i32;
        assert_eq!(job_result.status.unwrap().status, done_status);

        // Verify signature and message format
        let sig = Signature::try_from(&job_result.zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output =
            ResultWithMetadata::abi_decode_params(&job_result.result_with_metadata, false).unwrap();
        let raw_output = abi_decoded_output.4;
        let signing_payload = abi_encode_result_with_metadata(
            job_id.into(),
            &job_result.input,
            job_result.max_cycles,
            &job_result.program_id,
            &raw_output,
        )
        .unwrap();
        assert_eq!(job_result.result_with_metadata, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), job_result.input);

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
        assert_eq!(completed.jobID, expected_job_id);

        let get_balance_call = consumer_contract.getBalance(mock_user_address);
        let MockConsumer::getBalanceReturn { _0: balance } = get_balance_call.call().await.unwrap();
        assert_eq!(balance, expected_balance);
    }
    E2E::new().mock_consumer().run(test).await;
}
