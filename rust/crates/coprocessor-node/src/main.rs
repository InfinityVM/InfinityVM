//! The binary for running the server

use coprocessor_node::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    Cli::run().await
}
