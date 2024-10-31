//! CLI for interacting with the CLOB.

#[tokio::main]
async fn main() -> eyre::Result<()> {
    clob_client::cli::Cli::run().await
}
