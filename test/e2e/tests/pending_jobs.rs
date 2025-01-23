//! Test for the pending jobs endpoint.

use alloy::{
    network::EthereumWallet,
    primitives::{
        aliases::U256, keccak256, utils::eip191_hash_message, Address, Bytes, FixedBytes,
        PrimitiveSignature,
    },
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    signers::{local::PrivateKeySigner, Signer, SignerSync},
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

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn pending_jobs_works() {
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

        let futures: Vec<_> = (1..=3)
            .map(|nonce| {
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
                let request_signature =
                    random_user.sign_message_sync(&job_request_payload).unwrap();

                SubmitJobRequest {
                    request: job_request_payload,
                    signature: request_signature.into(),
                    offchain_input: vec![],
                    relay_strategy: RelayStrategy::Unordered as i32,
                }
            })
            .map(|r| (r, args.coprocessor_node.clone()))
            .map(async move |(r, mut c)| c.submit_job(r).await)
            .collect();

        // Wait for the jobs to hit the coproc
        futures::future::join_all(futures).await;
    }

    E2E::new().mock_consumer().run(test).await;
}
