//! The binary for running the server

use ivm_coprocessor_node::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    let _guards = zkvm_tracing::init_logging().unwrap();

    Cli::run().await
}
