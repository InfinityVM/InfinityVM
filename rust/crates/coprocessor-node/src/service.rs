use proto::{
    coprocessor_node_server::CoprocessorNode as CoprocessorNodeTrait, GetResultRequest,
    GetResultResponse, SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest,
    SubmitProgramResponse,
};
use tonic::{Request, Response, Status};

/// gRPC service server
#[derive(Debug, Default)]
pub struct CoprocessorNodeServerInner;

#[tonic::async_trait]
impl CoprocessorNodeTrait for CoprocessorNodeServerInner {
    /// SubmitJob defines the gRPC method for submitting a coprocessing job.
    async fn submit_job(
        &self,
        request: Request<SubmitJobRequest>,
    ) -> Result<Response<SubmitJobResponse>, Status> {
        let req = request.into_inner();
        let job = req.job.ok_or_else(|| Status::invalid_argument("missing job"))?;

        // verify fields
        if job.max_cycles == 0 {
            return Err(Status::invalid_argument("job max cycles must be positive"));
        }

        if job.id == 0 {
            return Err(Status::invalid_argument("job ID must be positive"));
        }

        if job.contract_address.is_empty() {
            return Err(Status::invalid_argument("job contract address must not be empty"));
        }

        if job.program_verifying_key.is_empty() {
            return Err(Status::invalid_argument("job program verification key must not be empty"));
        }

        // TODO: Implement executor in Rust
        // executor.submit_job(job)

        Ok(Response::new(SubmitJobResponse { job_id: job.id }))
    }
    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        request: Request<GetResultRequest>,
    ) -> std::result::Result<Response<GetResultResponse>, Status> {
        let req = request.into_inner();
        if req.job_id == 0 {
            return Err(Status::invalid_argument("job ID must be positive"));
        }

        // TODO: Implement executor in Rust
        // let job = executor.get_job(job_id)

        Ok(Response::new(GetResultResponse { job: None }))
    }
    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program verification key.
    async fn submit_program(
        &self,
        request: Request<SubmitProgramRequest>,
    ) -> std::result::Result<Response<SubmitProgramResponse>, Status> {
        let req = request.into_inner();
        if req.program_elf.is_empty() {
            return Err(Status::invalid_argument("program elf must not be empty"));
        }

        // TODO: Implement executor in Rust
        // let verifying_key = executor.submit_elf(req.program_elf, req.vm_type);

        Ok(Response::new(SubmitProgramResponse { program_verifying_key: vec![] }))
    }
}
