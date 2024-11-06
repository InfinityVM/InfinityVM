//! Load testing for the coprocessor node
use ivm_abi::get_job_id;
use alloy::primitives::Address;
use contracts::get_default_deploy_info;
use goose::prelude::*;
use mock_consumer::MOCK_CONSUMER_MAX_CYCLES;
use ivm_proto::{GetResultRequest, GetResultResponse};
use std::env;
use ivm_test_utils::{create_and_sign_offchain_request, get_signers};

/// Get the Anvil IP address env var.
pub fn anvil_ip() -> String {
    env::var("ANVIL_IP").unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Get the Anvil port env var.
pub fn anvil_port() -> String {
    env::var("ANVIL_PORT").unwrap_or_else(|_| "8545".to_string())
}

/// Get the coprocessor gateway IP addressenv var.
pub fn coprocessor_gateway_ip() -> String {
    env::var("COPROCESSOR_GATEWAY_IP").unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Get the coprocessor gateway port env var.
pub fn coprocessor_gateway_port() -> String {
    env::var("COPROCESSOR_GATEWAY_PORT").unwrap_or_else(|_| "8080".to_string())
}

/// Get the max cycles env var.
pub fn max_cycles() -> u64 {
    match env::var("MAX_CYCLES") {
        Ok(max_cycles) => max_cycles.parse().unwrap_or(MOCK_CONSUMER_MAX_CYCLES),
        Err(_) => MOCK_CONSUMER_MAX_CYCLES, // Default value if MAX_CYCLES is not set
    }
}

/// Get the consumer address env var. If not set, try to get the consumer address from the deploy
/// info. If that fails, use the default consumer address.
pub fn consumer_addr() -> String {
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

/// Get the wait until job completed env var. If not set, default to true.
pub fn should_wait_until_job_completed() -> bool {
    match env::var("WAIT_UNTIL_JOB_COMPLETED") {
        Ok(enabled) => enabled.to_lowercase() == "true",
        Err(_) => true,
    }
}

/// Get the number of users env var. If not set, default to 10.
pub fn num_users() -> usize {
    env::var("NUM_USERS").ok().and_then(|v| v.parse().ok()).unwrap_or(10)
}

/// Get the report file name env var. If not set, default to "report.html".
pub fn report_file_name() -> String {
    env::var("REPORT_FILE_NAME").ok().unwrap_or_else(|| "report.html".to_string())
}

/// Get the startup time env var. If not set, default to 10 seconds.
pub fn startup_time() -> usize {
    env::var("STARTUP_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(10)
}

/// Get the run time env var. If not set, default to 20 seconds.
pub fn run_time() -> usize {
    env::var("RUN_TIME").ok().and_then(|v| v.parse().ok()).unwrap_or(20)
}

/// Create and sign an ABI-encoded offchain request for a given nonce and program ID.
pub async fn get_offchain_request(
    nonce: u64,
    program_id: &[u8],
    onchain_input: &[u8],
) -> (Vec<u8>, Vec<u8>) {
    let consumer_addr = Address::parse_checksummed(consumer_addr(), None).expect("Valid address");
    let signers = get_signers(6);
    let offchain_signer = signers[5].clone();

    create_and_sign_offchain_request(
        nonce,
        max_cycles(),
        consumer_addr,
        onchain_input,
        program_id,
        offchain_signer,
        &[],
    )
    .await
}

/// Wait until a job is completed by the coprocessor node.
pub async fn wait_until_job_completed(user: &mut GooseUser, nonce: u64) {
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

/// Get the status of a job from the coprocessor node.
async fn get_result_status(
    user: &mut GooseUser,
    job_id: [u8; 32],
) -> Result<i64, Box<dyn std::error::Error>> {
    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let response = user.post_json("/v1/coprocessor_node/get_result", &get_result_request).await?;
    let get_result_response: GetResultResponse = response.response?.json().await?;

    match get_result_response.job_result {
        Some(job_result) => match job_result.status {
            Some(status) => Ok(status.status.into()),
            None => Ok(0),
        },
        None => Ok(0),
    }
}

/// Intensity level for the intensity test.
#[derive(Debug)]
pub enum IntensityLevel {
    /// Light intensity: 10 hash rounds
    Light,
    /// Medium intensity: 50 hash rounds
    Medium,
    /// Heavy intensity: 500 hash rounds
    Heavy,
}

/// Defaults to Medium if not set or if an invalid value is provided.
pub fn get_intensity_level() -> IntensityLevel {
    match env::var("INTENSITY_LEVEL")
        .unwrap_or_else(|_| "medium".to_string())
        .to_lowercase()
        .as_str()
    {
        "light" => IntensityLevel::Light,
        "heavy" => IntensityLevel::Heavy,
        _ => IntensityLevel::Medium,
    }
}

/// Get the number of hash rounds for the current intensity level.
pub fn intensity_hash_rounds() -> u32 {
    match get_intensity_level() {
        IntensityLevel::Light => 10,
        IntensityLevel::Medium => 50,
        IntensityLevel::Heavy => 500,
    }
}
