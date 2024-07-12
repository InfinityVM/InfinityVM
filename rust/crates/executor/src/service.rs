//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Signature},
    signers::Signer,
};
use kvdb::{DBTransaction, KeyValueDB};
use proto::{
    CreateElfRequest, CreateElfResponse, ExecuteRequest, ExecuteResponse, JobInputs, VmType,
};
use sha2::{Digest, Sha256};
use std::marker::Send;
use zkvm::Zkvm;

use alloy_sol_types::{sol, SolType};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
}

///  The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug)]
pub(crate) struct ZkvmExecutorService<S, D> {
    signer: S,
    chain_id: Option<u64>,
    db: D,
}

impl<S, D> ZkvmExecutorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: KeyValueDB,
{
    pub(crate) const fn new(signer: S, chain_id: Option<u64>, db: D) -> Self {
        Self { signer, chain_id, db }
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

    fn vm(&self, vm_type: i32) -> Result<Box<dyn Zkvm + Send>, tonic::Status> {
        let vm_type =
            VmType::try_from(vm_type).map_err(|_| tonic::Status::internal("invalid vm type"))?;
        let vm: Box<dyn Zkvm + Send> = match vm_type {
            VmType::Risc0 => Box::new(zkvm::Risc0),
            VmType::Sp1 => Box::new(zkvm::Sp1),
        };

        Ok(vm)
    }

    fn write_elf(
        &self,
        vm_type: i32,
        verifying_key: &[u8],
        program_elf: Vec<u8>,
    ) -> Result<(), tonic::Status> {
        let key = Sha256::digest(verifying_key);

        let mut tx = DBTransaction::new();
        tx.put_vec(vm_type as u32, key.as_slice(), program_elf);

        self.db
            .write(tx)
            .map_err(|e| tonic::Status::internal(format!("failed to write to db: {e}")))
    }

    fn read_elf(&self, vm_type: i32, verifying_key: &[u8]) -> Result<Vec<u8>, tonic::Status> {
        let key = Sha256::digest(verifying_key).to_vec();

        self.db
            .get(vm_type as u32, &key)
            .map_err(|e| tonic::Status::internal(format!("failed to read from db: {e}")))?
            .ok_or_else(|| {
                tonic::Status::invalid_argument("could not find program ELF with verifying key")
            })
    }
}

#[tonic::async_trait]
impl<S, D> proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: KeyValueDB + 'static,
{
    async fn execute(
        &self,
        request: tonic::Request<ExecuteRequest>,
    ) -> Result<tonic::Response<ExecuteResponse>, tonic::Status> {
        let msg = request.into_inner();
        let inputs = msg.inputs.expect("todo");

        let vm = self.vm(inputs.vm_type)?;
        let program_elf = self.read_elf(inputs.vm_type, &inputs.program_verifying_key)?;

        if !vm.is_correct_verifying_key(&program_elf, &inputs.program_verifying_key).expect("todo")
        {
            return Err(tonic::Status::invalid_argument("bad verifying key"));
        }

        let raw_output = vm
            .execute(&program_elf, &inputs.program_input, inputs.max_cycles)
            .map_err(|e| format!("zkvm execute error: {e:?}"))
            .map_err(tonic::Status::invalid_argument)?;

        let result_with_metadata = abi_encode_result_with_metadata(&inputs, &raw_output);

        let zkvm_operator_signature = self
            .sign_message(&result_with_metadata)
            .await
            .map_err(|e| format!("signing error: {e:?}"))
            .map_err(tonic::Status::internal)?;
        let response = ExecuteResponse {
            inputs: Some(inputs),
            result_with_metadata,
            zkvm_operator_address: self.address_checksum_bytes(),
            zkvm_operator_signature,
            raw_output,
        };

        Ok(tonic::Response::new(response))
    }

    async fn create_elf(
        &self,
        tonic_request: tonic::Request<CreateElfRequest>,
    ) -> Result<tonic::Response<CreateElfResponse>, tonic::Status> {
        let request = tonic_request.into_inner();

        let vm = self.vm(request.vm_type)?;

        let verifying_key = vm
            .derive_verifying_key(&request.program_elf)
            .map_err(|_| tonic::Status::invalid_argument("failed to derive verifying key"))?;

        self.write_elf(request.vm_type, &verifying_key, request.program_elf)?;

        let response = CreateElfResponse { verifying_key };
        Ok(tonic::Response::new(response))
    }
}

/// The payload that gets signed to signify that the zkvm executor has faithfully
/// executed the job. Also the result payload the job manager contract expects.
///
/// tuple(JobID,ProgramInputHash,MaxCycles,VerifyingKey,RawOutput)
type ResultWithMetadata = sol! {
    tuple(uint32,bytes32,uint64,bytes,bytes)
};

fn abi_encode_result_with_metadata(i: &JobInputs, raw_output: &[u8]) -> Vec<u8> {
    let program_input_hash = keccak256(&i.program_input);
    ResultWithMetadata::abi_encode(&(
        i.job_id,
        program_input_hash,
        i.max_cycles,
        &i.program_verifying_key,
        raw_output,
    ))
}
