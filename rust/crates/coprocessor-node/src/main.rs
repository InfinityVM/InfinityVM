//! The binary for running the server

use coprocessor_node::cli::Cli;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .init();

    Cli::run().await
}
