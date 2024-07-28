//! The binary for running the server

use coprocessor_node::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    // TODO (Maanav): Is this the correct way to add logging?
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .init();

    Cli::run().await
}
