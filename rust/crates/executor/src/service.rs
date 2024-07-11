//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Signature},
    signers::Signer,
};
use kvdb::KeyValueDB;
use proto::{
    CreateElfRequest, CreateElfResponse, ExecuteRequest, ExecuteResponse, JobInputs, VmType,
};
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
    _db: D,
}

impl<S, D> ZkvmExecutorService<S, D>
where
    S: Signer<Signature> + Send + Sync + 'static,
    D: KeyValueDB,
{
    pub(crate) const fn new(signer: S, chain_id: Option<u64>, db: D) -> Self {
        Self { signer, chain_id, _db: db }
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

        let vm:Box<dyn Zkvm + Send> = match VmType::try_from(inputs.vm_type)
            .map_err(|_| tonic::Status::internal("invalid vm type"))?
        {
            VmType::Risc0 => Box::new(zkvm::Risc0),
            VmType::Sp1 => Box::new(zkvm::Sp1),
        };

        if !vm
            .is_correct_verifying_key(&msg.program_elf, &inputs.program_verifying_key)
            .expect("todo")
        {
            return Err(tonic::Status::invalid_argument("bad verifying key"));
        }

        let raw_output = vm
            .execute(&msg.program_elf, &inputs.program_input, inputs.max_cycles)
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
        _request: tonic::Request<CreateElfRequest>,
    ) -> Result<tonic::Response<CreateElfResponse>, tonic::Status> {
        unimplemented!()
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
