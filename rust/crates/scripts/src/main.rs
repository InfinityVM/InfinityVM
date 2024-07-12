//! Script for generating ABI-encoded response + signature to the coprocessor contract tests
use scripts::signer::ResultSigner;

#[tokio::main]
async fn main() {
    ResultSigner::run().await
}
