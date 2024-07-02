//! The binary for running the server

use server::Cli;

#[tokio::main]
async fn main() -> Result<(), tonic::transport::Error> {
    Cli::run().await
}
