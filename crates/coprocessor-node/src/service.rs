use std::sync::Arc;

use crate::job_processor::JobProcessorService;
use abi::OffchainJobRequest;
use alloy::{
    primitives::{keccak256, Address, Bytes, Signature}, signers::Signer, sol_types::SolType
};
use base64::{prelude::BASE64_STANDARD, Engine};
use db::tables::{get_job_id, Job, RequestType};
use proto::{
    coprocessor_node_server::CoprocessorNode as CoprocessorNodeTrait, GetResultRequest,
    GetResultResponse, JobResult, JobStatus, JobStatusType, SubmitJobRequest, SubmitJobResponse,
    SubmitProgramRequest, SubmitProgramResponse, SubmitStatefulJobRequest,
    SubmitStatefulJobResponse,
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

        let OffchainJobRequest {
            nonce,
            max_cycles,
            consumer,
            program_id,
            program_input,
            program_state_hash: _,
        } = OffchainJobRequest::abi_decode(&req.request, false)
            .map_err(|_| Status::invalid_argument("invalid ABI-encoding of job request"))?;

        validate_job_request(max_cycles, consumer, program_id.clone(), req.signature.clone())?;
        let job_id = get_job_id(nonce, consumer);

        // TODO: Make contract calls to verify nonce, signature, etc. on job request
        // [ref: https://github.com/Ethos-Works/InfinityVM/issues/168]

        info!(job_id = ?job_id, "new job request");

        let job = Job {
            id: job_id,
            nonce,
            max_cycles,
            consumer_address: consumer.to_vec(),
            program_id: program_id.to_vec(),
            input: program_input.to_vec(),
            program_state: vec![],
            request_type: RequestType::Offchain(req.signature),
            result_with_metadata: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
        };

        self.job_processor
            .submit_job(job)
            .await
            .map_err(|e| Status::internal(format!("failed to submit job: {e}")))?;

        Ok(Response::new(SubmitJobResponse { job_id: job_id.to_vec() }))
    }
    /// SubmitStatefulJob defines the gRPC method for submitting a stateful coprocessing job.
    #[instrument(name = "coprocessor_submit_stateful_job", skip(self, request), err(Debug))]
    async fn submit_stateful_job(
        &self,
        request: Request<SubmitStatefulJobRequest>,
    ) -> Result<Response<SubmitStatefulJobResponse>, Status> {
        let req = request.into_inner();

        let OffchainJobRequest {
            nonce,
            max_cycles,
            consumer,
            program_id,
            program_input,
            program_state_hash,
        } = OffchainJobRequest::abi_decode(&req.request, false)
            .map_err(|_| Status::invalid_argument("invalid ABI-encoding of job request"))?;

        let state_hash = keccak256(req.program_state.clone());
        if state_hash != program_state_hash {
            return Err(Status::invalid_argument("program state hash does not match"));
        }

        validate_job_request(max_cycles, consumer, program_id.clone(), req.signature.clone())?;
        let job_id = get_job_id(nonce, consumer);

        // TODO: Make contract calls to verify nonce, signature, etc. on job request
        // [ref: https://github.com/Ethos-Works/InfinityVM/issues/168]

        info!(job_id = ?job_id, "new job request");

        let job = Job {
            id: job_id,
            nonce,
            max_cycles,
            consumer_address: consumer.to_vec(),
            program_id: program_id.to_vec(),
            input: program_input.to_vec(),
            program_state: req.program_state,
            request_type: RequestType::Offchain(req.signature),
            result_with_metadata: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
        };

        self.job_processor
            .submit_job(job)
            .await
            .map_err(|e| Status::internal(format!("failed to submit job: {e}")))?;

        Ok(Response::new(SubmitStatefulJobResponse { job_id: job_id.to_vec() }))
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

                let job_result = match job {
                    Some(job) => {
                        let request_signature = match job.request_type {
                            RequestType::Onchain => vec![],
                            RequestType::Offchain(signature) => signature,
                        };

                        Some(JobResult {
                            id: job.id.to_vec(),
                            nonce: job.nonce,
                            program_id: job.program_id,
                            input: job.input,
                            consumer_address: job.consumer_address,
                            max_cycles: job.max_cycles,
                            request_signature,
                            result_with_metadata: job.result_with_metadata,
                            zkvm_operator_signature: job.zkvm_operator_signature,
                            status: Some(job.status),
                        })
                    }
                    None => return Err(Status::not_found("job not found")),
                };

                Ok(Response::new(GetResultResponse { job_result }))
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

        let program_id = self
            .job_processor
            .submit_elf(req.program_elf, req.vm_type)
            .await
            .map_err(|e| Status::internal(format!("failed to submit ELF: {e}")))?;

        info!(program_id = BASE64_STANDARD.encode(program_id.clone()), "new elf program");

        Ok(Response::new(SubmitProgramResponse { program_id }))
    }
}

/// Verify fields in a submitted job request. Used for both stateful
/// and stateless job requests.
pub fn validate_job_request(
    max_cycles: u64,
    consumer_address: Address,
    program_id: Bytes,
    signature: Vec<u8>,
) -> Result<(), Status> {
    // verify fields
    if max_cycles == 0 {
        return Err(Status::invalid_argument("job max cycles must be positive"));
    }

    if signature.is_empty() {
        return Err(Status::invalid_argument("job request signature must not be empty"));
    }

    if consumer_address.len() != 20 {
        return Err(Status::invalid_argument("contract address must be 20 bytes in length"));
    }

    if program_id.is_empty() {
        return Err(Status::invalid_argument("job program verification key must not be empty"));
    }

    Ok(())
}
