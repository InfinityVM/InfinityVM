//! Helper script to generate ABI-encoded response + signature for the coprocessor contract tests
use ivm_scripts::signer::RequestAndResultSigner;

#[tokio::main]
async fn main() {
    let instance =
        ivm_test_utils::IvmExecInstance::try_spawn(2020, Some("./log/ivm-exec".into())).unwrap();

    // println!("===== ONCHAIN RESULT =====");
    // RequestAndResultSigner::sign_onchain_result().await;

    // println!("===== OFFCHAIN RESULT =====");
    // RequestAndResultSigner::sign_offchain_result().await;

    // println!("===== JOB REQUEST =====");
    // RequestAndResultSigner::sign_job_request().await;

    std::thread::sleep(std::time::Duration::from_secs(30));
}
