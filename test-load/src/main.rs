//! Load testing for the coprocessor node
use abi::abi_encode_offchain_job_request;
use alloy::{primitives::Address, signers::Signer, sol_types::SolValue};
use db::tables::{get_job_id, Job, RequestType};
use goose::prelude::*;
use mock_consumer_methods::MOCK_CONSUMER_GUEST_ID;
use once_cell::sync::Lazy;
use proto::{GetResultRequest, JobStatus, JobStatusType, SubmitJobRequest};
use serde_json::Value;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use test_utils::get_signers;

// Global atomic counter for the nonce
static GLOBAL_NONCE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(2)); // We start at 2 because the first job is submitted in the setup for LoadtestGetResult

const MAX_CYCLES: u64 = 1_000_000;
const CONSUMER_ADDR: &str = "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6";

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
    let wait_until_completed = match std::env::var("WAIT_UNTIL_JOB_COMPLETED") {
        Ok(enabled) => enabled.to_lowercase() == "true",
        Err(_) => false,
    };

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

    if wait_until_completed {
        wait_until_job_completed(user, nonce).await;
        let total_duration = start_time.elapsed();
        println!("Total duration until job completed: {:?}", total_duration);
    }
    Ok(())
}

async fn loadtest_get_result(user: &mut GooseUser) -> TransactionResult {
    let consumer_addr: Address =
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");
    let job_id = get_job_id(1, consumer_addr);

    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let payload = serde_json::to_value(get_result_request).expect("Valid GetResultRequest");

    let _goose_metrics = user.post_json("/v1/coprocessor_node/get_result", &payload).await?;

    Ok(())
}

async fn submit_first_job(user: &mut GooseUser) -> TransactionResult {
    let (encoded_job_request, signature) = create_and_sign_offchain_request(1).await;

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
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");

    let job = Job {
        id: get_job_id(nonce, consumer_addr),
        nonce,
        max_cycles: MAX_CYCLES,
        // Need to use abi_encode_packed because the contract address
        // should not be zero-padded
        consumer_address: Address::abi_encode_packed(&consumer_addr),
        program_id: MOCK_CONSUMER_GUEST_ID.iter().flat_map(|&x| x.to_le_bytes()).collect(),
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

async fn wait_until_job_completed(user: &mut GooseUser, nonce: u64) {
    let consumer_addr: Address =
        Address::parse_checksummed(CONSUMER_ADDR, None).expect("Valid address");
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
    let body = response.response?.text().await?;

    let result: Value = serde_json::from_str(&body)?;
    let status = result["jobResult"]["status"]["status"].as_i64();
    Ok(status.unwrap_or(0)) // Return 0 if status is not found or not an i64
}
