//! Load testing for the coprocessor node
use abi::get_job_id;
use alloy::{primitives::Address, providers::ProviderBuilder};
use borsh::{BorshDeserialize, BorshSerialize};
use contracts::mock_consumer::MockConsumer;
use goose::prelude::*;
use load_test::{
    anvil_ip, anvil_port, consumer_addr, coprocessor_gateway_ip, coprocessor_gateway_port,
    get_intensity_level, get_offchain_request, get_offchain_request_for_intensity_test,
    intensity_iterations, intensity_work_per_iteration, num_users, report_file_name, run_time,
    should_wait_until_job_completed, startup_time, wait_until_job_completed,
};
use once_cell::sync::Lazy;
use proto::{GetResultRequest, SubmitJobRequest};
use std::sync::Mutex;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tokio::sync::OnceCell;

static GLOBAL_NONCE: Lazy<OnceCell<AtomicU64>> = Lazy::new(OnceCell::new);
static JOB_COMPLETION_TIMES: Lazy<Mutex<Vec<Duration>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

#[tokio::main]
async fn main() -> Result<(), GooseError> {
    dotenv::from_filename("./load-test/.env").ok();

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
        .register_scenario(
            scenario!("LoadtestIntensityTest")
                .set_wait_time(Duration::from_secs(1), Duration::from_secs(3))?
                .register_transaction(transaction!(loadtest_intensity_test)),
        )
        .set_default(
            GooseDefault::Host,
            format!("http://{}:{}", coprocessor_gateway_ip(), coprocessor_gateway_port()).as_str(),
        )?
        .set_default(GooseDefault::Users, num_users())?
        .set_default(GooseDefault::ReportFile, report_file_name().as_str())?
        .set_default(GooseDefault::StartupTime, startup_time())?
        .set_default(GooseDefault::RunTime, run_time())?
        .execute()
        .await?;

    println!("Load test completed.");
    println!("Intensity test information:");
    println!("Running intensity test with level: {:?}", get_intensity_level());
    println!(
        "Iterations: {}, Hashes per iteration: {}",
        intensity_iterations(),
        intensity_work_per_iteration()
    );

    // Calculate and print average job completion time
    let job_completion_times = JOB_COMPLETION_TIMES.lock().unwrap();
    if !job_completion_times.is_empty() {
        let average_completion_time: Duration =
            job_completion_times.iter().sum::<Duration>() / job_completion_times.len() as u32;
        println!("Average job completion time: {:?}", average_completion_time);
    } else {
        println!("No jobs completed during the test.");
    }

    Ok(())
}

async fn loadtest_submit_job(user: &mut GooseUser) -> TransactionResult {
    let start_time = Instant::now();

    // Get the current nonce and increment it
    let nonce = GLOBAL_NONCE.get().unwrap().fetch_add(1, Ordering::SeqCst);

    let (encoded_job_request, signature) = get_offchain_request(nonce).await;

    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };
    let _goose_metrics =
        user.post_json("/v1/coprocessor_node/submit_job", &submit_job_request).await?;

    if should_wait_until_job_completed() {
        wait_until_job_completed(user, nonce).await;
        let total_duration = start_time.elapsed();
        println!("Job {} completed in {:?}", nonce, total_duration);
        // Add the job completion time to the vector
        JOB_COMPLETION_TIMES.lock().unwrap().push(total_duration);
    }
    Ok(())
}

async fn loadtest_get_result(user: &mut GooseUser) -> TransactionResult {
    let consumer_addr: Address =
        Address::parse_checksummed(consumer_addr(), None).expect("Valid address");
    let job_id = get_job_id(1, consumer_addr);

    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let _goose_metrics =
        user.post_json("/v1/coprocessor_node/get_result", &get_result_request).await?;

    Ok(())
}

/// Submit the first job to the coprocessor node so that
/// `loadtest_get_result()` is guaranteed to get the result of
/// this job. Only need to do this if the next nonce is 1.
pub async fn submit_first_job() -> Result<(), Box<dyn std::error::Error>> {
    let nonce = GLOBAL_NONCE.get().unwrap().fetch_add(1, Ordering::SeqCst);

    if nonce == 1 {
        let (encoded_job_request, signature) = get_offchain_request(nonce).await;

        let submit_job_request = SubmitJobRequest {
            request: encoded_job_request,
            signature,
            offchain_input: Vec::new(),
            state: Vec::new(),
        };

        let client = reqwest::Client::new();
        let _response = client
            .post(format!(
                "http://{}:{}/v1/coprocessor_node/submit_job",
                coprocessor_gateway_ip(),
                coprocessor_gateway_port()
            ))
            .json(&submit_job_request)
            .send()
            .await?;

        println!("First job submitted with nonce 1");
    }

    Ok(())
}

#[derive(BorshSerialize, BorshDeserialize)]
struct IntensityInput {
    iterations: u32,
    work_per_iteration: u32,
}

async fn loadtest_intensity_test(user: &mut GooseUser) -> TransactionResult {
    let start_time = Instant::now();

    let nonce = GLOBAL_NONCE.get().unwrap().fetch_add(1, Ordering::SeqCst);

    let intensity_input = IntensityInput {
        iterations: intensity_iterations(),
        work_per_iteration: intensity_work_per_iteration(),
    };
    let encoded_intensity = borsh::to_vec(&intensity_input).unwrap();

    let (encoded_job_request, signature) =
        get_offchain_request_for_intensity_test(nonce, &encoded_intensity).await;

    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };
    let _goose_metrics =
        user.post_json("/v1/coprocessor_node/submit_job", &submit_job_request).await?;

    if should_wait_until_job_completed() {
        wait_until_job_completed(user, nonce).await;
        let total_duration = start_time.elapsed();
        println!("Intensity test job {} completed in {:?}", nonce, total_duration);
        // Add the job completion time to the vector
        JOB_COMPLETION_TIMES.lock().unwrap().push(total_duration);
    }
    Ok(())
}
