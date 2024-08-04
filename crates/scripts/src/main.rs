//! Helper script to generate ABI-encoded response + signature for the coprocessor contract tests
use scripts::signer::ResultSigner;

#[tokio::main]
async fn main() {
    ResultSigner::run().await
}
