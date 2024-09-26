//! The binary for running the server

use coprocessor_node::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    // let _guards = zkvm_tracing::init_logging().unwrap();

    console_subscriber::init();

    Cli::run().await
}
