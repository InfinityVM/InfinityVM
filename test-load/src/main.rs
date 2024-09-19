//! Load test for the coprocessor node
use goose::{goose::GooseResponse, prelude::*};
use serde_json::json;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use once_cell::sync::Lazy;
use alloy::{
    primitives::{hex, keccak256, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use abi::abi_encode_offchain_job_request;
use db::tables::{get_job_id, Job, RequestType};
use proto::{JobStatus, JobStatusType};
use test_utils::get_signers;
use std::time::Duration;

// Global atomic counter for the nonce
static GLOBAL_NONCE: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(1));

const MAX_CYCLES: u64 = 1_000_000;
const CONSUMER_ADDR: &str = "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6";
const PROGRAM_ID: &[u8] = &[38, 97, 129, 246, 1, 9, 102, 56, 121, 187, 170, 57, 163, 102, 31, 208, 122, 142, 221, 113, 246, 162, 114, 4, 239, 24, 213, 94, 45, 195, 127, 233];

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    GooseAttack::initialize()?
        .register_scenario(scenario!("LoadtestTransactions")
            .set_wait_time(Duration::from_secs(1), Duration::from_secs(3))?
            .register_transaction(transaction!(loadtest_submit_job))
        )
        .execute()
        .await?;

    Ok(())
}

async fn loadtest_submit_job(user: &mut GooseUser) -> TransactionResult {
    // Get the current nonce and increment it
    let nonce = GLOBAL_NONCE.fetch_add(1, Ordering::SeqCst);

    let consumer_addr: Address = Address::parse_checksummed(CONSUMER_ADDR, None).unwrap();

    let job = Job {
        id: get_job_id(nonce, consumer_addr),
        nonce: nonce,
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
    let job_params = (&job).try_into().unwrap();
    let encoded_job_request = abi_encode_offchain_job_request(job_params);

    let signers = get_signers(6);
    let offchain_signer = signers[5].clone();
    let signature = offchain_signer.sign_message(&encoded_job_request).await.unwrap();

    let payload = json!({
        "request": encoded_job_request.iter().map(|&b| b as u64).collect::<Vec<u64>>(),
        "signature": signature.as_bytes().iter().map(|&b| b as u64).collect::<Vec<u64>>(),
        "offchainInput": Vec::<u64>::new(),
        "state": Vec::<u64>::new()
    });

    let _goose_metrics: GooseResponse = user.post_json("/v1/coprocessor_node/submit_job", &payload)
        .await?;

    Ok(())
}

async fn loadtest_get_result(user: &mut GooseUser) -> TransactionResult {
    let job_id = vec![5,188,43,223,98,136,178,224,47,78,102,129,175,34,220,98,116,251,224,221,123,169,160,117,225,67,48,4,21,123,170,61];
    
    let payload = json!({
        "jobId": job_id
    });

    let _goose_metrics = user.post_json("/v1/coprocessor_node/get_result", &payload)
        .await?;

    Ok(())
}
