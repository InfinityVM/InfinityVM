//! The binary for running the server

use executor::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).compact().init();

    Cli::run().await
}
