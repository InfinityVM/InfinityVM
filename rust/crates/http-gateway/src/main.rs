//! The binary for running the server

use http_gateway::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    Cli::run().await
}
