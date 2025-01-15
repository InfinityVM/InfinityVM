#![allow(missing_docs)]

use alloy::{
    network::EthereumWallet,
    primitives::{
        aliases::U256, keccak256, utils::eip191_hash_message, Address, Bytes, FixedBytes,
        PrimitiveSignature,
    },
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    signers::{local::PrivateKeySigner, Signer},
    sol,
    sol_types::{SolEvent, SolValue},
};
use e2e::{Args, E2E};
use ivm_abi::{
    abi_encode_offchain_job_request, abi_encode_offchain_result_with_metadata,
    abi_encode_result_with_metadata, get_job_id, JobParams, OffchainResultWithMetadata,
    ResultWithMetadata,
};
use ivm_contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
use ivm_mock_consumer::MOCK_CONSUMER_MAX_CYCLES;
use ivm_proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, JobStatusType, RelayStrategy,
    SubmitJobRequest, SubmitProgramRequest, VmType,
};
use mock_consumer_programs::{MOCK_CONSUMER_ELF, MOCK_CONSUMER_PROGRAM_ID};
use tokio::task::JoinSet;

type MockConsumerOut = sol!((Address, U256));

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn web2_job_submission_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let mock = args.mock_consumer.unwrap();
        let anvil = args.anvil;
        let program_id = MOCK_CONSUMER_PROGRAM_ID;
        let mock_user_address = Address::repeat_byte(69);

        let random_user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
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
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock.mock_consumer, &consumer_provider);

        // Submit job to coproc node
        let nonce = 1;
        let job_id = get_job_id(nonce, mock.mock_consumer);
        let offchain_input_hash = keccak256(vec![]);
        let job_params = JobParams {
            nonce,
            max_cycles: MOCK_CONSUMER_MAX_CYCLES,
            consumer_address: mock.mock_consumer.abi_encode_packed().try_into().unwrap(),
            onchain_input: &mock_user_address.abi_encode(),
            program_id: &program_id,
            offchain_input_hash: offchain_input_hash.into(),
        };

        // Add signature from user on job request
        let job_request_payload = abi_encode_offchain_job_request(job_params);
        let request_signature = random_user.sign_message(&job_request_payload).await.unwrap();

        let job_request = SubmitJobRequest {
            request: job_request_payload,
            signature: request_signature.into(),
            offchain_input: vec![],
            relay_strategy: RelayStrategy::Unordered as i32,
        };
        let submit_job_response =
            args.coprocessor_node.submit_job(job_request).await.unwrap().into_inner();
        assert_eq!(submit_job_response.job_id, job_id);

        // wait for the job to be processed and relayed
        tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

        let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let job_result = get_result_response.job_result.unwrap();

        // Verify the job status
        assert_eq!(job_result.status.unwrap().status(), JobStatusType::Relayed);

        // Verify the relay tx hash is not empty
        assert!(!job_result.relay_tx_hash.is_empty());

        // Verify signature and message format
        let sig = PrimitiveSignature::try_from(&job_result.zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output =
            OffchainResultWithMetadata::abi_decode(&job_result.result_with_metadata, false)
                .unwrap();

        let raw_output = abi_decoded_output.raw_output;
        let signing_payload = abi_encode_offchain_result_with_metadata(
            job_id,
            keccak256(&job_result.onchain_input),
            offchain_input_hash,
            job_result.max_cycles,
            &job_result.program_id,
            &raw_output,
            vec![],
        );
        assert_eq!(job_result.result_with_metadata, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), job_result.onchain_input);

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
        let get_inputs_call = consumer_contract.getOnchainInputForJob(FixedBytes(job_id));
        let MockConsumer::getOnchainInputForJobReturn { _0: inputs } =
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

/// Perform job submission in parallel with a JoinSet. Because job submission
/// is triggered by async grpc handler functions in the coprocessor, submitting
/// them in parallel more closely matches a real load.
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn web2_parallel_job_submission_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let mock = args.mock_consumer.unwrap();
        let anvil = args.anvil;
        let program_id = MOCK_CONSUMER_PROGRAM_ID;
        let mock_user_address = Address::repeat_byte(69);

        let random_user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        // Create 4 job requests
        let mut jobs = vec![];
        for nonce in 1..5 {
            let job_id = get_job_id(nonce, mock.mock_consumer);
            let offchain_input_hash = keccak256(vec![]);
            let job_params = JobParams {
                nonce,
                max_cycles: MOCK_CONSUMER_MAX_CYCLES,
                consumer_address: mock.mock_consumer.abi_encode_packed().try_into().unwrap(),
                onchain_input: &mock_user_address.abi_encode(),
                program_id: &program_id,
                offchain_input_hash: offchain_input_hash.into(),
            };

            let job_request_payload = abi_encode_offchain_job_request(job_params);
            let request_signature = random_user.sign_message(&job_request_payload).await.unwrap();

            let job_request = SubmitJobRequest {
                request: job_request_payload,
                signature: request_signature.into(),
                offchain_input: vec![],
                relay_strategy: RelayStrategy::Ordered as i32,
            };
            jobs.push((job_id, job_request));
        }

        // Submit jobs to coproc node
        let mut join_set = JoinSet::new();
        for (job_id, job_request) in jobs.clone() {
            let endpoint = args.coprocessor_node_endpoint.clone();
            join_set.spawn(async move {
                let mut coprocessor_node =
                    CoprocessorNodeClient::connect(endpoint.clone()).await.unwrap();
                let submit_job_response =
                    coprocessor_node.submit_job(job_request).await.unwrap().into_inner();
                assert_eq!(submit_job_response.job_id, job_id);
            });
        }

        // Wait for all jobs to be submitted
        join_set.join_all().await;

        // Give the node enough time to process and relay the jobs
        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

        // Assert that all jobs were successfully relayed.
        for (job_id, _) in jobs {
            let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
            let get_result_response =
                args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
            let job_result = get_result_response.job_result.unwrap();
            assert_eq!(job_result.status.unwrap().status(), JobStatusType::Relayed);
        }
    }
    E2E::new().mock_consumer().run(test).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn event_job_created_coprocessor_node_mock_consumer_e2e() {
    async fn test(mut args: Args) {
        let mock = args.mock_consumer.unwrap();
        let anvil = args.anvil;
        let program_id = MOCK_CONSUMER_PROGRAM_ID;
        let mock_user_address = Address::repeat_byte(69);

        let random_user: PrivateKeySigner = anvil.anvil.keys()[5].clone().into();
        let random_user_wallet = EthereumWallet::from(random_user);

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
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
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
        let get_result_response =
            args.coprocessor_node.get_result(get_result_request).await.unwrap().into_inner();
        let job_result = get_result_response.job_result.unwrap();

        // Verify the job status
        let relayed_status = JobStatusType::Relayed as i32;
        assert_eq!(job_result.status.unwrap().status, relayed_status);

        // Verify the relay tx hash is not empty
        assert!(!job_result.relay_tx_hash.is_empty());

        // Check saved height
        let current_block_number = consumer_provider.get_block_number().await.unwrap();
        let saved_height = ivm_db::get_last_block_height(args.db.clone()).await.unwrap().unwrap();
        assert_ne!(current_block_number, 0);
        assert_eq!(current_block_number + 1, saved_height);

        // Verify signature and message format
        let sig = PrimitiveSignature::try_from(&job_result.zkvm_operator_signature[..]).unwrap();
        let abi_decoded_output =
            ResultWithMetadata::abi_decode(&job_result.result_with_metadata, false).unwrap();
        let raw_output = abi_decoded_output.raw_output;
        let signing_payload = abi_encode_result_with_metadata(
            job_id.into(),
            keccak256(&job_result.onchain_input),
            job_result.max_cycles,
            &job_result.program_id,
            &raw_output,
        );
        assert_eq!(job_result.result_with_metadata, signing_payload);
        let recovered1 = sig.recover_address_from_msg(&signing_payload[..]).unwrap();
        assert_eq!(recovered1, anvil.coprocessor_operator.address());

        // confirm we are hashing as expected
        let hash = eip191_hash_message(&signing_payload);
        let recovered2 = sig.recover_address_from_prehash(&hash).unwrap();
        assert_eq!(recovered2, anvil.coprocessor_operator.address());

        // Verify input
        assert_eq!(Address::abi_encode(&mock_user_address), job_result.onchain_input);

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
