//! The binary for running the IVM coprocessor node.

use ivm_coproc::cli::Cli;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[cfg(not(feature = "jemalloc"))]
#[global_allocator]
static GLOBAL_ALLOCATOR: std::alloc::System = std::alloc::System;

// We give tokio just 2 worker threads because we want to maximize resources
// for job execution worker thread and the writer thread.
#[tokio::main(worker_threads = 2)]
async fn main() {
    let _guards = ivm_tracing::init_logging().unwrap();

    if let Err(e) = Cli::run().await {
        println!("Error: {}", e);
        std::process::exit(1);
    }
}
