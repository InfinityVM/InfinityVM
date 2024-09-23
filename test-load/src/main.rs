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
use test_load::{
    consumer_addr, get_offchain_request, num_users, report_file_name, run_time,
    should_wait_until_job_completed, startup_time, submit_first_job, wait_until_job_completed,
};
use test_utils::get_signers;

// Global atomic counter for the nonce
static GLOBAL_NONCE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(2)); // We start at 2 because the first job is submitted in the setup for LoadtestGetResult

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

    let (encoded_job_request, signature) = get_offchain_request(nonce).await;

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
