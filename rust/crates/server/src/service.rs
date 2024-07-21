use proto::{
    GetResultRequest, GetResultResponse, SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest,
    SubmitProgramResponse,
};

/// Use me!
#[derive(Debug)]
pub struct Server;

impl Server {
    /// Create a new server.
    pub fn new() -> Self {
        Self
    }
}

#[tonic::async_trait]
impl proto::service_server::Service for Server {
    /// SubmitJob defines the gRPC method for submitting a coprocessing job.
    async fn submit_job(
        &self,
        _request: tonic::Request<SubmitJobRequest>,
    ) -> std::result::Result<tonic::Response<SubmitJobResponse>, tonic::Status> {
        println!("submit_job");
        Ok(tonic::Response::new(SubmitJobResponse {
            job_id: 0,
        }))
    }
    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        _request: tonic::Request<GetResultRequest>,
    ) -> std::result::Result<tonic::Response<GetResultResponse>, tonic::Status> {
        println!("get_result");
        Ok(tonic::Response::new(GetResultResponse {
            job: None,
        }))
    }
    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program verification key.
    async fn submit_program(
        &self,
        _request: tonic::Request<SubmitProgramRequest>,
    ) -> std::result::Result<tonic::Response<SubmitProgramResponse>, tonic::Status> {
        println!("submit_program");
        Ok(tonic::Response::new(SubmitProgramResponse {
            program_verifying_key: vec![],
        }))
    }
}
