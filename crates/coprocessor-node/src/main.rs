//! The binary for running the server

use ivm_coprocessor_node::cli::Cli;

// We give tokio just 2 worker threads because we want to maximize resources
// for job execution worker thread and the writer thread.
#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), impl std::error::Error> {
    let _guards = zkvm_tracing::init_logging().unwrap();

    Cli::run().await
}
