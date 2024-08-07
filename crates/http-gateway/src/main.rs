//! Binary for running the http gateway.

use http_gateway::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    zkvm_tracing::init_logging().unwrap();

    Cli::run().await
}
