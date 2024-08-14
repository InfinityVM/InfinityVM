use std::sync::Arc;

use crate::job_processor::JobProcessorService;
use alloy::{primitives::Signature, signers::Signer};
use base64::{prelude::BASE64_STANDARD, Engine};
use proto::{
    coprocessor_node_server::CoprocessorNode as CoprocessorNodeTrait, GetResultRequest,
    GetResultResponse, SubmitJobRequest, SubmitJobResponse, SubmitProgramRequest,
    SubmitProgramResponse,
};
use reth_db::Database;
use tonic::{Request, Response, Status};
use tracing::{info, instrument};

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
    #[instrument(name = "coprocessor_submit_job", skip(self, request), err(Debug))]
    async fn submit_job(
        &self,
        request: Request<SubmitJobRequest>,
    ) -> Result<Response<SubmitJobResponse>, Status> {
        let req = request.into_inner();
        let job = req.job.ok_or_else(|| Status::invalid_argument("missing job"))?;
        let id = job.id.clone();

        // verify fields
        if job.max_cycles == 0 {
            return Err(Status::invalid_argument("job max cycles must be positive"));
        }

        let job_id_array: Result<[u8; 32], _> = id.clone().try_into();
        if job_id_array.is_err() {
            return Err(Status::invalid_argument("job ID must be 32 bytes in length"));
        }

        if job.contract_address.is_empty() {
            return Err(Status::invalid_argument("job contract address must not be empty"));
        }

        if job.program_verifying_key.is_empty() {
            return Err(Status::invalid_argument("job program verification key must not be empty"));
        }
        info!(job_id = ?job.id, "new job request");

        self.job_processor
            .submit_job(job)
            .await
            .map_err(|e| Status::internal(format!("failed to submit job: {e}")))?;

        Ok(Response::new(SubmitJobResponse { job_id: id }))
    }
    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        request: Request<GetResultRequest>,
    ) -> Result<Response<GetResultResponse>, Status> {
        let req = request.into_inner();
        let job_id_array: Result<[u8; 32], _> = req.job_id.clone().try_into();

        match job_id_array {
            Ok(job_id) => {
                let job = self
                    .job_processor
                    .get_job(job_id)
                    .await
                    .map_err(|e| Status::internal(format!("failed to get job: {e}")))?;

                Ok(Response::new(GetResultResponse { job }))
            }
            Err(_) => Err(Status::invalid_argument("job ID must be 32 bytes in length")),
        }
    }
    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program verification key.
    #[instrument(name = "coprocessor_submit_program", skip(self, request), err(Debug))]
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

        info!(verifying_key = BASE64_STANDARD.encode(verifying_key.clone()), "new elf program");

        Ok(Response::new(SubmitProgramResponse { program_verifying_key: verifying_key }))
    }
}
