use std::sync::Arc;

use crate::job_processor::JobProcessorService;
use alloy::{primitives::Signature, signers::Signer};
use proto::{
    coprocessor_node_server::CoprocessorNode as CoprocessorNodeTrait, GetResultRequest,
    GetResultResponse, SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest,
    SubmitProgramResponse,
};
use reth_db::Database;
use tonic::{Request, Response, Status};

/// gRPC service server
#[derive(Debug)]
pub struct CoprocessorNodeServerInner<S, D> {
    // TODO (Maanav): should we use `DatabaseEnv` instead of a generic `D`?
    /// Job processor service
    pub job_processor: Arc<JobProcessorService<S, D>>,
}

#[tonic::async_trait]
impl<S, D> CoprocessorNodeTrait for CoprocessorNodeServerInner<S, D>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// SubmitJob defines the gRPC method for submitting a coprocessing job.
    async fn submit_job(
        &self,
        request: Request<SubmitJobRequest>,
    ) -> Result<Response<SubmitJobResponse>, Status> {
        let req = request.into_inner();
        let job = req.job.ok_or_else(|| Status::invalid_argument("missing job"))?;
        let nonce = job.nonce.ok_or_else(|| Status::invalid_argument("missing nonce"))?;
        let contract_address = job.contract_address.clone();
        // let id = job.id;

        // verify fields
        if job.max_cycles == 0 {
            return Err(Status::invalid_argument("job max cycles must be positive"));
        }

        if job.id.is_some() {
            return Err(Status::invalid_argument("job ID must not be set"));
        }

        if job.request_signature.is_empty() {
            return Err(Status::invalid_argument("job request signature must not be empty"));
        }

        if contract_address.is_empty() {
            return Err(Status::invalid_argument("job contract address must not be empty"));
        }

        if job.program_verifying_key.is_empty() {
            return Err(Status::invalid_argument("job program verification key must not be empty"));
        }

        self.job_processor
            .submit_job(job)
            .await
            .map_err(|e| Status::internal(format!("failed to submit job: {e}")))?;

        Ok(Response::new(SubmitJobResponse { nonce: nonce, contract_address: contract_address }))
    }
    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        request: Request<GetResultRequest>,
    ) -> Result<Response<GetResultResponse>, Status> {
        let req = request.into_inner();
        if req.job_id == 0 {
            return Err(Status::invalid_argument("job ID must be positive"));
        }

        let job = self
            .job_processor
            .get_job_for_id(req.job_id)
            .await
            .map_err(|e| Status::internal(format!("failed to get job: {e}")))?;

        Ok(Response::new(GetResultResponse { job }))
    }
    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program verification key.
    async fn submit_program(
        &self,
        request: Request<SubmitProgramRequest>,
    ) -> Result<Response<SubmitProgramResponse>, Status> {
        let req = request.into_inner();
        if req.program_elf.is_empty() {
            return Err(Status::invalid_argument("program elf must not be empty"));
        }

        let verifying_key = self
            .job_processor
            .submit_elf(req.program_elf, req.vm_type)
            .await
            .map_err(|e| Status::internal(format!("failed to submit ELF: {e}")))?;

        Ok(Response::new(SubmitProgramResponse { program_verifying_key: verifying_key }))
    }
}
