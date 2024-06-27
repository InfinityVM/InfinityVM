//! Better then your server.

// use proto::zkvm_executor_server::{ZkvmExecutor, ZkvmExecutorServer};
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};

/// Protobuf generated types
pub mod proto {
    tonic::include_proto!("zkvm_executor");
}

///  The implementation of the ZkvmExecutor trait
#[derive(Debug)]
pub struct ZkvmExecutorService;

#[tonic::async_trait]
impl proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService {
    async fn execute(
        &self,
        _request: tonic::Request<ExecuteRequest>,
    ) -> Result<tonic::Response<ExecuteResponse>, tonic::Status> {
        unimplemented!()
    }
}
