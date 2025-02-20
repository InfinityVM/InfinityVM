//! Helper script to generate ABI-encoded response + signature for the coprocessor contract tests
use ivm_scripts::signer::RequestAndResultSigner;

#[tokio::main]
async fn main() {
    ivm_test_utils::IvmExecInstance::spawn(2020).unwrap();

    println!("===== ONCHAIN RESULT =====");
    RequestAndResultSigner::sign_onchain_result().await;

    println!("===== OFFCHAIN RESULT =====");
    RequestAndResultSigner::sign_offchain_result().await;

    println!("===== JOB REQUEST =====");
    RequestAndResultSigner::sign_job_request().await;
}
