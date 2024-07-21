//! The server

use proto::{
    GetResultRequest, GetResultResponse, SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest,
    SubmitProgramResponse,
};

pub mod cli;

/// Use me!
#[derive(Debug)]
pub struct Server;

#[tonic::async_trait]
impl proto::service_server::Service for Server {
    /// SubmitJob defines the gRPC method for submitting a coprocessing job.
    async fn submit_job(
        &self,
        _request: tonic::Request<SubmitJobRequest>,
    ) -> std::result::Result<tonic::Response<SubmitJobResponse>, tonic::Status> {
        todo!()
    }
    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        _request: tonic::Request<GetResultRequest>,
    ) -> std::result::Result<tonic::Response<GetResultResponse>, tonic::Status> {
        todo!()
    }
    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program verification key.
    async fn submit_program(
        &self,
        _request: tonic::Request<SubmitProgramRequest>,
    ) -> std::result::Result<tonic::Response<SubmitProgramResponse>, tonic::Status> {
        todo!()
    }
}
