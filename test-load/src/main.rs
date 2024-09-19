//! Load testing for the coprocessor node
use abi::abi_encode_offchain_job_request;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, FixedBytes},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use contracts::mock_consumer::MockConsumer;
use db::tables::{get_job_id, Job, RequestType};
use goose::{goose::GooseResponse, prelude::*};
use once_cell::sync::Lazy;
use proto::{JobStatus, JobStatusType};
use serde_json::json;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use test_utils::get_signers;

// Global atomic counter for the nonce
static GLOBAL_NONCE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(2)); // We start at 2 because the first job is submitted in the setup for LoadtestGetResult

const MAX_CYCLES: u64 = 1_000_000;
const CONSUMER_ADDR: &str = "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6";
const PROGRAM_ID: &[u8] = &[
    38, 97, 129, 246, 1, 9, 102, 56, 121, 187, 170, 57, 163, 102, 31, 208, 122, 142, 221, 113, 246,
    162, 114, 4, 239, 24, 213, 94, 45, 195, 127, 233,
];

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_scenario(
            scenario!("LoadtestSubmitJob")
                .set_wait_time(Duration::from_secs(1), Duration::from_secs(3))?
                .register_transaction(transaction!(loadtest_submit_job)),
        )
        .register_scenario(
            scenario!("LoadtestGetResult")
                .register_transaction(transaction!(submit_first_job).set_on_start()) // We want to submit the first job first before getting the result of this job
                .register_transaction(transaction!(loadtest_get_result)),
        )
        .execute()
        .await?;

    Ok(())
}

async fn loadtest_submit_job(user: &mut GooseUser) -> TransactionResult {
    let wait_until_relay = match std::env::var("WAIT_UNTIL_RELAY") {
        Ok(enabled) => enabled.to_lowercase() == "true",
        Err(_) => false,
    };

    let start_time = Instant::now();

    // Get the current nonce and increment it
    let nonce = GLOBAL_NONCE.fetch_add(1, Ordering::SeqCst);

    let (encoded_job_request, signature) = create_and_sign_offchain_request(nonce).await;

    let payload = json!({
        "request": encoded_job_request,
        "signature": signature,
        "offchainInput": Vec::<u8>::new(),
        "state": Vec::<u8>::new()
    });

    let _goose_metrics: GooseResponse =
        user.post_json("/v1/coprocessor_node/submit_job", &payload).await?;

    if wait_until_relay {
        wait_until_result_relayed(nonce).await;
        let total_duration = start_time.elapsed();
        println!("Total duration until result is relayed: {:?}", total_duration);
    }
    Ok(())
}

async fn loadtest_get_result(user: &mut GooseUser) -> TransactionResult {
    let consumer_addr: Address =
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");
    let job_id = get_job_id(1, consumer_addr);

    let payload = json!({
        "jobId": job_id
    });

    let _goose_metrics = user.post_json("/v1/coprocessor_node/get_result", &payload).await?;

    Ok(())
}

async fn submit_first_job(user: &mut GooseUser) -> TransactionResult {
    let (encoded_job_request, signature) = create_and_sign_offchain_request(1).await;

    let payload = json!({
        "request": encoded_job_request,
        "signature": signature,
        "offchainInput": Vec::<u64>::new(),
        "state": Vec::<u64>::new()
    });

    let _goose_metrics = user.post_json("/v1/coprocessor_node/submit_job", &payload).await?;

    Ok(())
}

async fn create_and_sign_offchain_request(nonce: u64) -> (Vec<u8>, Vec<u8>) {
    let consumer_addr: Address =
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");

    let job = Job {
        id: get_job_id(nonce, consumer_addr),
        nonce,
        max_cycles: MAX_CYCLES,
        // Need to use abi_encode_packed because the contract address
        // should not be zero-padded
        consumer_address: Address::abi_encode_packed(&consumer_addr),
        program_id: PROGRAM_ID.to_vec(),
        onchain_input: Address::abi_encode(&consumer_addr),
        offchain_input: vec![],
        state: vec![],
        request_type: RequestType::Offchain(vec![]),
        result_with_metadata: vec![],
        zkvm_operator_signature: vec![],
        status: JobStatus {
            status: JobStatusType::Pending as i32,
            failure_reason: None,
            retries: 0,
        },
    };
    let job_params = (&job).try_into().expect("Valid job");
    let encoded_job_request = abi_encode_offchain_job_request(job_params);

    let signers = get_signers(6);
    let offchain_signer = signers[5].clone();
    let signature =
        offchain_signer.sign_message(&encoded_job_request).await.expect("Signing should work");

    (encoded_job_request, signature.as_bytes().to_vec())
}

async fn wait_until_result_relayed(nonce: u64) {
    let random_user: PrivateKeySigner = get_signers(6)[5].clone();
    let random_user_wallet = EthereumWallet::from(random_user);

    let consumer_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(random_user_wallet)
        .on_http(url::Url::parse("http://127.0.0.1:60420").expect("Valid URL"));
    let consumer_addr: Address =
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");
    let consumer_contract = MockConsumer::new(consumer_addr, &consumer_provider);

    let job_id = get_job_id(nonce, consumer_addr);
    let mut inputs = vec![0u8; 32];
    // If the inputs are all zero, then the job is not yet relayed
    while inputs.iter().all(|&x| x == 0) {
        let get_inputs_call = consumer_contract.getOnchainInputForJob(FixedBytes(job_id));
        match get_inputs_call.call().await {
            Ok(MockConsumer::getOnchainInputForJobReturn { _0: result }) => {
                inputs = result.to_vec();
                if inputs.iter().all(|&x| x == 0) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
            Err(e) => eprintln!("Error calling getOnchainInputForJob: {:?}", e),
        }
    }
}
