//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Address, Signature},
    signers::Signer,
};
use base64::prelude::*;
use proto::VmType;
use std::marker::Send;
use zkvm::Zkvm;

use alloy::{sol, sol_types::SolType};
use tracing::{error, info};

/// Zkvm executor errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error with Alloy signer
    #[error("signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
    /// Invalid VM type
    #[error("invalid VM type")]
    InvalidVmType,
    /// Could not find ELF for VM type
    #[error("could not find elf for vm={0}")]
    ElfNotFound(String),
    /// Invalid verifying key
    #[error("bad verifying key {0}")]
    InvalidVerifyingKey(String),
    /// Could not derive verifying key
    #[error("failed to derive verifying key {0}")]
    VerifyingKeyDerivationFailed(String),
    /// Error with zkvm execution
    #[error("zkvm execute error: {0}")]
    ZkvmExecuteFailed(#[from] zkvm::Error),
    /// Error converting job ID
    #[error("job id conversion error")]
    JobIdConversion,
}

/// The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug, Clone)]
pub struct ZkvmExecutorService<S> {
    signer: S,
}

/// Inputs provided for execution of a job
#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    /// The job ID (hash of nonce and consumer address)
    pub job_id: [u8; 32],
    /// CPU cycle limit for job execution
    pub max_cycles: u64,
    /// The ZK program verification key
    pub program_id: Vec<u8>,
    /// Program execution input
    pub input: Vec<u8>,
    /// ELF of the program to execute
    pub elf: Vec<u8>,
    /// VM type
    pub vm_type: VmType,
}

/// Outputs from the execution of a job
#[derive(Debug, Clone)]
pub struct ExecuteResponse {
    /// ABI-encoded result of job execution with metadata
    /// tuple(JobID,ProgramInputHash,MaxCycles,ProgramID,RawOutput)
    pub result_with_metadata: Vec<u8>,
    /// Signature of the operator that executed the job on the result with metadata
    pub zkvm_operator_signature: Vec<u8>,
}

impl<S> ZkvmExecutorService<S>
where
    S: Signer<Signature> + Send + Sync + 'static + Clone,
{
    /// Create a new zkvm executor service
    pub const fn new(signer: S) -> Self {
        Self { signer }
    }

    /// Returns the address of the signer
    pub fn signer_address(&self) -> Address {
        self.signer.address()
    }

    /// Returns an RLP encoded signature over `eip191_hash_message(msg)`
    async fn sign_message(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        self.signer.sign_message(msg).await.map(|sig| sig.as_bytes().to_vec()).map_err(Into::into)
    }

    /// Returns the VM and VM type (enum) for the given VM type (i32)
    fn vm(&self, vm_type: VmType) -> Result<Box<dyn Zkvm + Send>, Error> {
        let vm: Box<dyn Zkvm + Send> = match vm_type {
            VmType::Risc0 => Box::new(zkvm::Risc0),
            VmType::Sp1 => unimplemented!("https://github.com/Ethos-Works/InfinityVM/issues/120"),
        };

        Ok(vm)
    }

    /// Checks the verifying key, executes a program on the given inputs, and returns signed output.
    pub async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse, Error> {
        let base64_program_id = BASE64_STANDARD.encode(request.program_id.as_slice());
        let vm = self.vm(request.vm_type)?;
        info!(
            job_id = ?request.job_id.clone(),
            vm_type = request.vm_type.as_str_name(),
            program_id = base64_program_id,
            "new job received"
        );

        if !vm.is_correct_verifying_key(&request.elf, &request.program_id).expect("todo") {
            return Err(Error::InvalidVerifyingKey(format!(
                "bad verifying key {}",
                base64_program_id,
            )));
        }

        let raw_output = vm
            .execute(&request.elf, &request.input, request.max_cycles)
            .map_err(Error::ZkvmExecuteFailed)?;

        let result_with_metadata = abi_encode_result_with_metadata(&request, &raw_output)?;
        let zkvm_operator_signature = self.sign_message(&result_with_metadata).await?;

        info!(
            job_id = ?request.job_id,
            vm_type = request.vm_type.as_str_name(),
            program_id = base64_program_id,
            raw_output = BASE64_STANDARD.encode(raw_output.as_slice()),
            "job complete"
        );

        let response = ExecuteResponse { result_with_metadata, zkvm_operator_signature };

        Ok(response)
    }

    /// Derives and returns program ID (verifying key) for the
    /// given program ELF.
    pub async fn create_elf(&self, elf: Vec<u8>, vm_type: VmType) -> Result<Vec<u8>, Error> {
        let vm = self.vm(vm_type)?;

        let program_id = vm
            .derive_verifying_key(&elf)
            .map_err(|e| Error::VerifyingKeyDerivationFailed(e.to_string()))?;

        let base64_program_id = BASE64_STANDARD.encode(program_id.as_slice());

        info!(vm_type = vm_type.as_str_name(), program_id = base64_program_id, "new elf program");

        Ok(program_id)
    }
}

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,ProgramID,RawOutput)
pub type ResultWithMetadata = sol! {
    tuple(bytes32,bytes32,uint64,bytes,bytes)
};

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(
    req: &ExecuteRequest,
    raw_output: &[u8],
) -> Result<Vec<u8>, Error> {
    let program_input_hash = keccak256(&req.input);

    let job_id: [u8; 32] = req.job_id.clone().try_into().map_err(|_| Error::JobIdConversion)?;

    Ok(ResultWithMetadata::abi_encode_params(&(
        job_id,
        program_input_hash,
        req.max_cycles,
        &req.program_id,
        raw_output,
    )))
}
