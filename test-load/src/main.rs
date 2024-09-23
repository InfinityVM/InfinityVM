//! Load testing for the coprocessor node
use abi::{abi_encode_offchain_job_request, JobParams};
use alloy::{
    primitives::{keccak256, Address},
    signers::Signer,
    sol_types::SolValue,
};
use contracts::get_default_deploy_info;
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

// Global atomic counter for the nonce
static GLOBAL_NONCE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(2)); // We start at 2 because the first job is submitted in the setup for LoadtestGetResult

fn max_cycles() -> u64 {
    match env::var("MAX_CYCLES") {
        Ok(max_cycles) => max_cycles.parse().unwrap_or(1000000),
        Err(_) => 1000000, // Default value if MAX_CYCLES is not set
    }
}

fn consumer_addr() -> String {
    match env::var("CONSUMER_ADDR") {
        Ok(consumer_addr) => consumer_addr,
        Err(_) => {
            // If env var is not set, try to get the consumer address from the deploy info
            let deploy_info = get_default_deploy_info();
            match deploy_info {
                Ok(deploy_info) => deploy_info.mock_consumer.to_string(),
                Err(_) => "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6".to_string(), /* Default consumer address */
            }
        }
    }
}

fn should_wait_until_job_completed() -> bool {
    match env::var("WAIT_UNTIL_JOB_COMPLETED") {
        Ok(enabled) => enabled.to_lowercase() == "true",
        Err(_) => false,
    }
}

fn num_users() -> usize {
    env::var("NUM_USERS").ok().and_then(|v| v.parse().ok()).unwrap_or(10) // Default to 10 users if
                                                                          // not set or invalid
}

fn report_file_name() -> String {
    env::var("REPORT_FILE_NAME").ok().unwrap_or_else(|| "report.html".to_string())
}

fn startup_time() -> usize {
    env::var("STARTUP_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(10) // Default to 10
                                                                             // seconds if not set
                                                                             // or invalid
}

fn run_time() -> usize {
    env::var("RUN_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(20) // Default to 20 seconds if
                                                                         // not set or invalid
}

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    dotenv::from_filename("./test-load/.env").ok();

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
        .set_default(GooseDefault::Users, num_users())?
        .set_default(GooseDefault::ReportFile, report_file_name().as_str())?
        .set_default(GooseDefault::StartupTime, startup_time())?
        .set_default(GooseDefault::RunTime, run_time())?
        .execute()
        .await?;

    Ok(())
}

async fn loadtest_submit_job(user: &mut GooseUser) -> TransactionResult {
    let start_time = Instant::now();

    // Get the current nonce and increment it
    let nonce = GLOBAL_NONCE.fetch_add(1, Ordering::SeqCst);

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

async fn submit_first_job(user: &mut GooseUser) -> TransactionResult {
    let nonce = 1;
    let (encoded_job_request, signature) = create_and_sign_offchain_request(nonce).await;

    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };
    let payload = serde_json::to_value(submit_job_request).expect("Valid SubmitJobRequest");

    let _goose_metrics = user.post_json("/v1/coprocessor_node/submit_job", &payload).await?;

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
