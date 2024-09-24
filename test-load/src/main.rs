//! Load testing for the coprocessor node
use abi::{abi_encode_offchain_job_request, JobParams};
use alloy::{
    primitives::{keccak256, Address},
    providers::ProviderBuilder,
    signers::Signer,
    sol_types::SolValue,
};
use contracts::{get_default_deploy_info, mock_consumer::MockConsumer};
use db::tables::get_job_id;
use goose::prelude::*;
use mock_consumer_methods::MOCK_CONSUMER_GUEST_ID;
use once_cell::sync::Lazy;
use proto::{GetResultRequest, GetResultResponse, SubmitJobRequest};
use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use test_utils::get_signers;
use tokio::sync::OnceCell;

static GLOBAL_NONCE: Lazy<OnceCell<AtomicU64>> = Lazy::new(OnceCell::new);

mod config;

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    dotenv::from_filename("./test-load/.env").ok();

    // Initialize GLOBAL_NONCE
    GLOBAL_NONCE.get_or_init(initialize_global_nonce).await;

    // Submit the first job so loadtest_get_result() is guaranteed to get the result of this job
    let _ = submit_first_job().await;

    GooseAttack::initialize()?
        .register_scenario(
            scenario!("LoadtestSubmitJob")
                .set_wait_time(Duration::from_secs(1), Duration::from_secs(3))?
                .register_transaction(transaction!(loadtest_submit_job)),
        )
        .register_scenario(
            scenario!("LoadtestGetResult").register_transaction(transaction!(loadtest_get_result)),
        )
        .set_default(
            GooseDefault::Host,
            format!("http://{}:{}", coprocessor_gateway_ip(), coprocessor_gateway_port()).as_str(),
        )?
        .set_default(GooseDefault::Users, config::num_users())?
        .set_default(GooseDefault::ReportFile, config::report_file_name().as_str())?
        .set_default(GooseDefault::StartupTime, config::startup_time())?
        .set_default(GooseDefault::RunTime, config::run_time())?
        .execute()
        .await?;

    Ok(())
}

async fn initialize_global_nonce() -> AtomicU64 {
    let anvil_ip = anvil_ip();
    let anvil_port = anvil_port();
    let provider = ProviderBuilder::new().with_recommended_fillers().on_http(
        url::Url::parse(format!("http://{anvil_ip}:{anvil_port}").as_str()).expect("Valid URL"),
    );

    let mock_consumer_address = consumer_addr().parse().unwrap();
    let consumer_contract = MockConsumer::new(mock_consumer_address, &provider);

    let nonce = consumer_contract.getNextNonce().call().await.unwrap()._0;

    AtomicU64::new(nonce)
}

async fn submit_first_job() -> Result<(), Box<dyn std::error::Error>> {
    let nonce = GLOBAL_NONCE.get().unwrap().fetch_add(1, Ordering::SeqCst);

    if nonce == 1 {
        let (encoded_job_request, signature) = create_and_sign_offchain_request(nonce).await;

        let submit_job_request = SubmitJobRequest {
            request: encoded_job_request,
            signature,
            offchain_input: Vec::new(),
            state: Vec::new(),
        };
        let payload = serde_json::to_value(submit_job_request)?;

        let client = reqwest::Client::new();
        let _response = client
            .post(format!(
                "http://{}:{}/v1/coprocessor_node/submit_job",
                coprocessor_gateway_ip(),
                coprocessor_gateway_port()
            ))
            .json(&payload)
            .send()
            .await?;

        println!("First job submitted with nonce 1");
    }

    Ok(())
}

async fn loadtest_submit_job(user: &mut GooseUser) -> TransactionResult {
    let start_time = Instant::now();

    // Get the current nonce and increment it
    let nonce = GLOBAL_NONCE.get().unwrap().fetch_add(1, Ordering::SeqCst);

    let (encoded_job_request, signature) = create_and_sign_offchain_request(nonce).await;

    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };
    let payload = serde_json::to_value(submit_job_request).expect("Valid SubmitJobRequest");

    let _goose_metrics = user.post_json("/v1/coprocessor_node/submit_job", &payload).await?;

    if should_wait_until_job_completed() {
        wait_until_job_completed(user, nonce).await;
        let total_duration = start_time.elapsed();
        println!("Total duration until job completed: {:?}", total_duration);
    }
    Ok(())
}

async fn loadtest_get_result(user: &mut GooseUser) -> TransactionResult {
    let consumer_addr: Address =
        Address::parse_checksummed(consumer_addr(), None).expect("Valid address");
    let job_id = get_job_id(1, consumer_addr);

    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let payload = serde_json::to_value(get_result_request).expect("Valid GetResultRequest");

    let _goose_metrics = user.post_json("/v1/coprocessor_node/get_result", &payload).await?;

    Ok(())
}

async fn create_and_sign_offchain_request(nonce: u64) -> (Vec<u8>, Vec<u8>) {
    let consumer_addr: Address =
        Address::parse_checksummed(consumer_addr(), None).expect("Valid address");

    let encoded_consumer_addr = Address::abi_encode(&consumer_addr);
    let job_params = JobParams {
        nonce,
        max_cycles: max_cycles(),
        // Need to use abi_encode_packed because the contract address
        // should not be zero-padded
        consumer_address: Address::abi_encode_packed(&consumer_addr)
            .try_into()
            .expect("Valid consumer address"),
        onchain_input: encoded_consumer_addr.as_slice(),
        program_id: &MOCK_CONSUMER_GUEST_ID
            .iter()
            .flat_map(|&x| x.to_le_bytes())
            .collect::<Vec<u8>>(),
        offchain_input_hash: *keccak256([]),
        state_hash: *keccak256([]),
    };

    let encoded_job_request = abi_encode_offchain_job_request(job_params);

    let signers = get_signers(6);
    let offchain_signer = signers[5].clone();
    let signature =
        offchain_signer.sign_message(&encoded_job_request).await.expect("Signing should work");

    (encoded_job_request, signature.as_bytes().to_vec())
}

async fn wait_until_job_completed(user: &mut GooseUser, nonce: u64) {
    let consumer_addr: Address =
        Address::parse_checksummed(consumer_addr(), None).expect("Valid address");
    let job_id = get_job_id(nonce, consumer_addr);

    loop {
        match get_result_status(user, job_id).await {
            Ok(status) => {
                // 2 is the status code for a completed job
                if status == 2 {
                    break;
                }
            }
            Err(e) => eprintln!("Error getting result: {:?}", e),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}

async fn get_result_status(
    user: &mut GooseUser,
    job_id: [u8; 32],
) -> Result<i64, Box<dyn std::error::Error>> {
    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let payload = serde_json::to_value(get_result_request)?;

    let response = user.post_json("/v1/coprocessor_node/get_result", &payload).await?;
    let get_result_response: GetResultResponse = response.response?.json().await?;

    match get_result_response.job_result {
        Some(job_result) => match job_result.status {
            Some(status) => Ok(status.status.into()),
            None => Ok(0),
        },
        None => Ok(0),
    }
}
