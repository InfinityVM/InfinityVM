//! Test for the pending jobs endpoint.

use alloy::{
    primitives::{keccak256, Address},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, SignerSync},
    sol_types::{SolValue},
};
use e2e::{Args, E2E};
use ivm_abi::{abi_encode_offchain_job_request, get_job_id, JobParams};
use ivm_contracts::mock_consumer::MockConsumer;
use ivm_mock_consumer::MOCK_CONSUMER_MAX_CYCLES;
use ivm_proto::{
    GetPendingJobsRequest, RelayStrategy, SubmitJobRequest, SubmitProgramRequest, VmType,
};
use mock_consumer_programs::{MOCK_CONSUMER_ELF, MOCK_CONSUMER_PROGRAM_ID};

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

        let pending_jobs_request =
            GetPendingJobsRequest { consumer_address: mock.mock_consumer.to_vec() };
        let pending = args
            .coprocessor_node
            .get_pending_jobs(pending_jobs_request.clone())
            .await
            .unwrap()
            .into_inner();
        assert!(pending.pending_jobs.is_empty());

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
                    // We want ordered to ensure this works
                    relay_strategy: RelayStrategy::Ordered as i32,
                }
            })
            .map(|r| (r, args.coprocessor_node.clone()))
            .map(async move |(r, mut c)| c.submit_job(r).await)
            .collect();

        // Wait for the jobs to hit the coproc
        futures::future::join_all(futures).await;

        let pending = args
            .coprocessor_node
            .get_pending_jobs(pending_jobs_request.clone())
            .await
            .unwrap()
            .into_inner();
        assert_eq!(pending.pending_jobs, vec![1, 2, 3]);
        dbg!(pending.pending_jobs);

        // We should delete all pending jobs as they complete
        for i in 0..60 {
            let pending = args
                .coprocessor_node
                .get_pending_jobs(pending_jobs_request.clone())
                .await
                .unwrap()
                .into_inner();
            dbg!(&pending.pending_jobs, i);
            if pending.pending_jobs.is_empty() {
                return
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        panic!("waiting for over 60 seconds and still have pending jobs")
    }

    E2E::new().mock_consumer().run(test).await;
}
