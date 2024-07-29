//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Address, Signature},
    signers::Signer,
};
use base64::prelude::*;
use db::{self, tables::ElfWithMeta};
use proto::{
    CreateElfRequest, CreateElfResponse, ExecuteRequest, ExecuteResponse, JobInputs, VmType,
};
use reth_db::Database;
use std::{marker::Send, sync::Arc};
use zkvm::Zkvm;

use alloy_sol_types::{sol, SolType};
use tracing::{error, info, instrument};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
    #[error("invalid VM type")]
    InvalidVmType,
    #[error("failed reading elf: {0}")]
    ElfReadFailed(#[from] db::Error),
    #[error("failed writing elf: {0}")]
    ElfWriteFailed(String),
    #[error("could not find elf for vm={0}")]
    ElfNotFound(String),
    #[error("elf with verifying key {0} already exists")]
    ElfAlreadyExists(String),
    #[error("bad verifying key {0}")]
    InvalidVerifyingKey(String),
    #[error("failed to derive verifying key {0}")]
    VerifyingKeyDerivationFailed(String),
    #[error("zkvm execute error: {0}")]
    ZkvmExecuteFailed(#[from] zkvm::Error),
}

///  The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug)]
pub(crate) struct ZkvmExecutorService<S, D> {
    signer: S,
    chain_id: Option<u64>,
    db: Arc<D>,
}

impl<S, D> ZkvmExecutorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: Database,
{
    pub(crate) const fn new(signer: S, chain_id: Option<u64>, db: Arc<D>) -> Self {
        Self { signer, chain_id, db }
    }

    pub(crate) fn signer_address(&self) -> Address {
        self.signer.address()
    }

    /// Checksum address (hex string), as bytes.
    fn address_checksum_bytes(&self) -> Vec<u8> {
        self.signer.address().to_checksum(self.chain_id).as_bytes().to_vec()
    }

    /// Returns an RLP encoded signature over `eip191_hash_message(msg)`
    async fn sign_message(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        self.signer
            .sign_message(msg)
            .await
            .map(|sig| {
                let mut out = Vec::with_capacity(sig.rlp_vrs_len());
                sig.write_rlp_vrs(&mut out);
                out
            })
            .map_err(Into::into)
    }

    fn vm(&self, vm_type: i32) -> Result<(Box<dyn Zkvm + Send>, VmType), Error> {
        let vm_type = VmType::try_from(vm_type).map_err(|_| Error::InvalidVmType)?;
        let vm: Box<dyn Zkvm + Send> = match vm_type {
            VmType::Risc0 => Box::new(zkvm::Risc0),
            VmType::Sp1 => unimplemented!("https://github.com/Ethos-Works/InfinityVM/issues/120"),
        };

        Ok((vm, vm_type))
    }

    async fn execute_handler(&self, request: ExecuteRequest) -> Result<ExecuteResponse, Error> {
        let inputs = request.inputs.expect("todo");

        let ElfWithMeta { vm_type, elf } =
            db::get_elf(self.db.clone(), &inputs.program_verifying_key)
                .map_err(Error::ElfReadFailed)?
                .ok_or_else(|| Error::ElfNotFound("could not find elf for".to_string()))?;

        let base64_verifying_key = BASE64_STANDARD.encode(inputs.program_verifying_key.as_slice());
        let (vm, vm_type) = self.vm(vm_type as i32)?;
        info!(
            inputs.job_id,
            vm_type = vm_type.as_str_name(),
            verifying_key = base64_verifying_key,
            "new job received"
        );

        if !vm.is_correct_verifying_key(&elf, &inputs.program_verifying_key).expect("todo") {
            return Err(Error::InvalidVerifyingKey(format!(
                "bad verifying key {}",
                base64_verifying_key,
            )));
        }

        let raw_output = vm
            .execute(&elf, &inputs.program_input, inputs.max_cycles)
            .map_err(Error::ZkvmExecuteFailed)?;

        let result_with_metadata = abi_encode_result_with_metadata(&inputs, &raw_output);

        let zkvm_operator_signature = self.sign_message(&result_with_metadata).await?;

        info!(
            inputs.job_id,
            vm_type = vm_type.as_str_name(),
            verifying_key = base64_verifying_key,
            raw_output = BASE64_STANDARD.encode(raw_output.as_slice()),
            "job complete"
        );

        let response = ExecuteResponse {
            inputs: Some(inputs),
            result_with_metadata,
            zkvm_operator_address: self.address_checksum_bytes(),
            zkvm_operator_signature,
            raw_output,
        };

        Ok(response)
    }

    async fn create_elf_handler(
        &self,
        request: CreateElfRequest,
    ) -> Result<CreateElfResponse, Error> {
        let (vm, vm_type) = self.vm(request.vm_type)?;

        let verifying_key = vm
            .derive_verifying_key(&request.program_elf)
            .map_err(|e| Error::VerifyingKeyDerivationFailed(e.to_string()))?;

        let base64_verifying_key = BASE64_STANDARD.encode(verifying_key.as_slice());
        if db::get_elf(self.db.clone(), &verifying_key).map_err(Error::ElfReadFailed)?.is_some() {
            return Err(Error::ElfAlreadyExists(format!(
                "elf with verifying key {} already exists",
                base64_verifying_key,
            )));
        }

        db::put_elf(self.db.clone(), vm_type, &verifying_key, request.program_elf)
            .map_err(|e| Error::ElfWriteFailed(e.to_string()))?;

        info!(
            vm_type = vm_type.as_str_name(),
            verifying_key = base64_verifying_key,
            "new elf program"
        );

        let response = CreateElfResponse { verifying_key };

        Ok(response)
    }
}

#[tonic::async_trait]
impl<S, D> proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: Database + 'static,
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
    tuple(uint32,bytes32,uint64,bytes,bytes)
};

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(i: &JobInputs, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.program_input);
    ResultWithMetadata::abi_encode_params(&(
        i.job_id,
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}
