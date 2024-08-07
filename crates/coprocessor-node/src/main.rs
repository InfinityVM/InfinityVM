//! The binary for running the server

use coprocessor_node::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    zkvm_tracing::init_logging().unwrap();

    Cli::run().await
}
