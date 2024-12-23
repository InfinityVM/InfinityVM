//! gRPC server handlers.

use crate::intake::IntakeHandlers;
use futures_util::future::try_future::TryFutureExt;
use alloy::{
    hex,
    primitives::{keccak256, PrimitiveSignature},
    signers::{Signer, SignerSync},
    sol_types::SolType,
};
use ivm_abi::{get_job_id, OffchainJobRequest};
use ivm_db::tables::{Job, RequestType};
use ivm_proto::{
    coprocessor_node_server::CoprocessorNode as CoprocessorNodeTrait, GetResultRequest,
    GetResultResponse, JobResult, JobStatus, JobStatusType, SubmitJobRequest, SubmitJobResponse,
    SubmitProgramRequest, SubmitProgramResponse,
};
use reth_db::Database;
use tonic::{Request, Response, Status};
use tracing::info;

/// gRPC service server
#[derive(Debug)]
pub struct CoprocessorNodeServerInner<S, D> {
    intake_service: IntakeHandlers<S, D>,
}

impl<S, D> CoprocessorNodeServerInner<S, D> {
    /// Create a new instance of [Self].
    pub const fn new(intake_service: IntakeHandlers<S, D>) -> Self {
        Self { intake_service }
    }
}

#[tonic::async_trait]
impl<S, D> CoprocessorNodeTrait for CoprocessorNodeServerInner<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// SubmitJob defines the gRPC method for submitting a coprocessing job.
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
            onchain_input,
            offchain_input_hash: offchain_input_hash_in_request,
        } = OffchainJobRequest::abi_decode(&req.request, false)
            .map_err(|_| Status::invalid_argument("invalid ABI-encoding of job request"))?;

        // verify fields
        if max_cycles == 0 {
            return Err(Status::invalid_argument("job max cycles must be positive"));
        }

        if req.signature.is_empty() {
            return Err(Status::invalid_argument("job request signature must not be empty"));
        }

        if consumer.len() != 20 {
            return Err(Status::invalid_argument("contract address must be 20 bytes in length"));
        }

        if program_id.is_empty() {
            return Err(Status::invalid_argument("job program ID must not be empty"));
        }

        let offchain_input_hash = keccak256(&req.offchain_input);
        if offchain_input_hash_in_request != offchain_input_hash {
            return Err(Status::invalid_argument("offchain input hash does not match"));
        }

        let job_id = get_job_id(nonce, consumer);

        // TODO: Make contract calls to verify nonce, signature, etc. on job request
        // [ref: https://github.com/InfinityVM/InfinityVM/issues/168]

        info!(job_id = hex::encode(job_id), "new job request");

        let relay_strategy = req.relay_strategy();
        let job = Job {
            id: job_id,
            nonce,
            max_cycles,
            consumer_address: consumer.to_vec(),
            program_id: program_id.to_vec(),
            onchain_input: onchain_input.to_vec(),
            offchain_input: req.offchain_input,
            request_type: RequestType::Offchain(req.signature),
            result_with_metadata: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
            relay_tx_hash: vec![],
            blobs_sidecar: None,
            relay_strategy,
        };

        self.intake_service
            .submit_job(job)
            .await
            .map_err(|e| Status::internal(format!("failed to submit job: {e}")))?;

        Ok(Response::new(SubmitJobResponse { job_id: job_id.to_vec() }))
    }

    /// GetResult defines the gRPC method for getting the result of a coprocessing
    /// job.
    async fn get_result(
        &self,
        request: Request<GetResultRequest>,
    ) -> Result<Response<GetResultResponse>, Status> {
        let req = request.into_inner();
        let job_id: [u8; 32] = req
            .job_id
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("job ID must be 32 bytes in length"))?;

        let job = self
            .intake_service
            .get_job(job_id)
            .await
            .map_err(|e| Status::internal(format!("failed to get job: {e}")))?;

        let job_result = job
            .map(|job| {
                let request_signature = match job.request_type {
                    RequestType::Onchain => vec![],
                    RequestType::Offchain(signature) => signature,
                };

                JobResult {
                    id: job.id.to_vec(),
                    nonce: job.nonce,
                    program_id: job.program_id,
                    onchain_input: job.onchain_input,
                    offchain_input_hash: keccak256(&job.offchain_input).as_slice().to_vec(),
                    consumer_address: job.consumer_address,
                    max_cycles: job.max_cycles,
                    request_signature,
                    result_with_metadata: job.result_with_metadata,
                    zkvm_operator_signature: job.zkvm_operator_signature,
                    status: Some(job.status),
                    relay_tx_hash: job.relay_tx_hash,
                }
            })
            .ok_or_else(|| Status::not_found("job not found"))?;

        Ok(Response::new(GetResultResponse { job_result: Some(job_result) }))
    }

    /// SubmitProgram defines the gRPC method for submitting a new program to
    /// generate a unique program ID (verification key).
    async fn submit_program(
        &self,
        request: Request<SubmitProgramRequest>,
    ) -> Result<Response<SubmitProgramResponse>, Status> {
        let req = request.into_inner();
        if req.program_elf.is_empty() {
            return Err(Status::invalid_argument("program elf must not be empty"));
        }

        // Deriving the program ID is expensive so we do it in a blocking task.
        let intake_service = self.intake_service.clone();
        let program_id = intake_service
            .submit_elf(req.program_elf, req.vm_type, req.program_id)
            .await
            .map_err(|e| Status::internal(format!("failed to submit ELF: {e}")))?;

        info!(program_id = hex::encode(program_id.clone()), "new elf program");

        Ok(Response::new(SubmitProgramResponse { program_id }))
    }
}
