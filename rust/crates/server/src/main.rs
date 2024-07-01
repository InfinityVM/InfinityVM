//! The binary for running the server

use server::Cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Cli::run().await;

    Ok(())
}
