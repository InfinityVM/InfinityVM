//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Address, Signature},
    signers::Signer,
};
use base64::prelude::*;
use proto::{
    CreateElfRequest, CreateElfResponse, ExecuteRequest, ExecuteResponse, JobInputs, RequestType,
    VmType,
};
use std::marker::Send;
use zkvm::Zkvm;

use alloy::{sol, sol_types::SolType};
use tracing::{error, info, instrument};

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
    /// Error parsing request type
    #[error("error parsing request type")]
    RequestTypeParse,
}

///  The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug, Clone)]
pub struct ZkvmExecutorService<S> {
    signer: S,
    chain_id: Option<u64>,
}

impl<S> ZkvmExecutorService<S>
where
    S: Signer<Signature> + Send + Sync + 'static + Clone,
{
    /// Create a new zkvm executor service
    pub const fn new(signer: S, chain_id: Option<u64>) -> Self {
        Self { signer, chain_id }
    }

    /// Returns the address of the signer
    pub fn signer_address(&self) -> Address {
        self.signer.address()
    }

    /// Checksum address (hex string), as bytes.
    fn address_checksum_bytes(&self) -> Vec<u8> {
        self.signer.address().to_checksum(self.chain_id).as_bytes().to_vec()
    }

    /// Returns an RLP encoded signature over `eip191_hash_message(msg)`
    async fn sign_message(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        self.signer.sign_message(msg).await.map(|sig| sig.as_bytes().to_vec()).map_err(Into::into)
    }

    /// Returns the VM and VM type (enum) for the given VM type (i32)
    fn vm(&self, vm_type: i32) -> Result<(Box<dyn Zkvm + Send>, VmType), Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;
        let vm: Box<dyn Zkvm + Send> = match vm_type {
            VmType::Risc0 => Box::new(zkvm::Risc0),
            VmType::Sp1 => unimplemented!("https://github.com/Ethos-Works/InfinityVM/issues/120"),
        };

        Ok((vm, vm_type))
    }

    /// Checks the verifying key, executes a program on the given inputs, and returns signed output.
    /// This handler can be called directly.
    pub async fn execute_handler(&self, request: ExecuteRequest) -> Result<ExecuteResponse, Error> {
        let inputs = request.inputs.expect("todo");
        let base64_program_id = BASE64_STANDARD.encode(inputs.program_id.as_slice());
        let (vm, vm_type) = self.vm(inputs.vm_type)?;
        info!(
            job_id = ?inputs.job_id.clone(),
            vm_type = vm_type.as_str_name(),
            program_id = base64_program_id,
            "new job received"
        );

        if !vm.is_correct_verifying_key(&inputs.program_elf, &inputs.program_id).expect("todo") {
            return Err(Error::InvalidVerifyingKey(format!(
                "bad verifying key {}",
                base64_program_id,
            )));
        }

        let raw_output = vm
            .execute(&inputs.program_elf, &inputs.program_input, inputs.max_cycles)
            .map_err(Error::ZkvmExecuteFailed)?;

        let result_with_metadata = match RequestType::try_from(inputs.request_type) {
            Ok(RequestType::Onchain) => abi_encode_result_with_metadata(&inputs, &raw_output)?,
            Ok(RequestType::Offchain) => {
                abi_encode_offchain_result_with_metadata(&inputs, &raw_output)?
            }
            Err(_) => {
                return Err(Error::RequestTypeParse);
            }
        };

        let zkvm_operator_signature = self.sign_message(&result_with_metadata).await?;

        info!(
            job_id = ?inputs.job_id.clone(),
            vm_type = vm_type.as_str_name(),
            program_id = base64_program_id,
            raw_output = BASE64_STANDARD.encode(raw_output.as_slice()),
            "job complete"
        );

        let response = ExecuteResponse {
            result_with_metadata,
            zkvm_operator_address: self.address_checksum_bytes(),
            zkvm_operator_signature,
        };

        Ok(response)
    }

    /// Derives and returns program ID (verifying key) for the
    /// given program ELF. This handler can be called directly.
    pub async fn create_elf_handler(
        &self,
        request: CreateElfRequest,
    ) -> Result<CreateElfResponse, Error> {
        let (vm, vm_type) = self.vm(request.vm_type)?;

        let program_id = vm
            .derive_verifying_key(&request.program_elf)
            .map_err(|e| Error::VerifyingKeyDerivationFailed(e.to_string()))?;

        let base64_program_id = BASE64_STANDARD.encode(program_id.as_slice());

        info!(vm_type = vm_type.as_str_name(), program_id = base64_program_id, "new elf program");

        let response = CreateElfResponse { program_id };

        Ok(response)
    }
}

#[tonic::async_trait]
impl<S> proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService<S>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
{
    #[instrument(skip(self, request), err(Debug))]
    async fn execute(
        &self,
        request: tonic::Request<ExecuteRequest>,
    ) -> Result<tonic::Response<ExecuteResponse>, tonic::Status> {
        let msg = request.into_inner();
        let response = self
            .execute_handler(msg)
            .await
            .map_err(|e| tonic::Status::internal(format!("error in execute {e}")))?;
        Ok(tonic::Response::new(response))
    }

    #[instrument(skip(self, tonic_request), err(Debug))]
    async fn create_elf(
        &self,
        tonic_request: tonic::Request<CreateElfRequest>,
    ) -> Result<tonic::Response<CreateElfResponse>, tonic::Status> {
        let request = tonic_request.into_inner();
        let response = self
            .create_elf_handler(request)
            .await
            .map_err(|e| tonic::Status::internal(format!("error in create_elf {e}")))?;
        Ok(tonic::Response::new(response))
    }
}

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
pub type ResultWithMetadata = sol! {
    tuple(bytes32,bytes32,uint64,bytes,bytes)
};

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed an offchain job. Also the result payload the job manager contract expects.
///
/// tuple(ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
pub type OffchainResultWithMetadata = sol! {
    tuple(bytes32,uint64,bytes,bytes)
};

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(i: &JobInputs, raw_output: &[u8]) -> Result<Vec<u8>, Error> {
    let program_input_hash = keccak256(&i.program_input);

    let job_id: [u8; 32] = i.job_id.clone().try_into().map_err(|_| Error::JobIdConversion)?;

    Ok(ResultWithMetadata::abi_encode_params(&(
        job_id,
        program_input_hash,
        i.max_cycles,
        &i.program_id,
        raw_output,
    )))
}

/// Returns an ABI-encoded offchain result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_offchain_result_with_metadata(
    i: &JobInputs,
    raw_output: &[u8],
) -> Result<Vec<u8>, Error> {
    let program_input_hash = keccak256(&i.program_input);

    Ok(OffchainResultWithMetadata::abi_encode_params(&(
        program_input_hash,
        i.max_cycles,
        &i.program_id,
        raw_output,
    )))
}
