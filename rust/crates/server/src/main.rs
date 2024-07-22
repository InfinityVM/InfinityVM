//! The binary for running the server

use server::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    Cli::run().await
}
