//! Helper script to generate ABI-encoded response + signature for the coprocessor contract tests
use scripts::signer::RequestAndResultSigner;

#[tokio::main]
async fn main() {
    println!("===== ONCHAIN RESULT =====");
    RequestAndResultSigner::sign_onchain_result().await;

    println!("===== JOB REQUEST =====");
    RequestAndResultSigner::sign_job_request().await;

    println!("===== OFFCHAIN RESULT =====");
    RequestAndResultSigner::sign_offchain_result().await;
}
