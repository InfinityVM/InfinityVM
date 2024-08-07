//! Binary for running the http gateway.

use http_gateway::Cli;

#[tokio::main]
async fn main() -> Result<(), impl std::error::Error> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .json()
        .with_writer(non_blocking)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .compact()
        .init();

    Cli::run().await
}
