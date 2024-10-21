//! CLI for interacting with the CLOB.

#[tokio::main]
async fn main() -> eyre::Result<()> {
    simple_client::cli::Cli::run().await
}
