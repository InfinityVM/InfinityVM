//! The binary for running the server

use server::proto::zkvm_executor_server::ZkvmExecutorServer;
use server::ZkvmExecutorService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:10000".parse().unwrap();

    let zkvm_executor = ZkvmExecutorService {};

    let server = ZkvmExecutorServer::new(zkvm_executor
	);

    tonic::transport::Server::builder()
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}
