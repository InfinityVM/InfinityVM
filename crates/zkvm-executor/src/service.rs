//! gRPC service implementation.

use alloy::{
    primitives::{keccak256, Address, FixedBytes, Signature},
    signers::Signer,
    sol,
    sol_types::SolValue,
};
use base64::prelude::*;
use proto::VmType;
use std::marker::Send;
use zkvm::Zkvm;

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
}

/// The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug, Clone)]
pub struct ZkvmExecutorService<S> {
    signer: S,
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
    /// Returns (`result_with_metadata`, `zkvm_operator_signature`)
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self,
        job_id: [u8; 32],
        max_cycles: u64,
        program_id: Vec<u8>,
        input: Vec<u8>,
        program_state: Vec<u8>,
        elf: Vec<u8>,
        vm_type: VmType,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let base64_program_id = BASE64_STANDARD.encode(program_id.as_slice());
        let vm = self.vm(vm_type)?;
        info!(
            ?job_id,
            vm_type = vm_type.as_str_name(),
            program_id = base64_program_id,
            "new job received"
        );

        if !vm.is_correct_verifying_key(&elf, &program_id).expect("todo") {
            return Err(Error::InvalidVerifyingKey(format!(
                "bad verifying key {}",
                base64_program_id,
            )));
        }

        let program_input_hash = keccak256(&input);
        let program_state_hash = keccak256(&program_state);
        let raw_output = match program_state.is_empty() {
            true => tokio::task::spawn_blocking(move || {
                vm.execute(&elf, &input, max_cycles).map_err(Error::ZkvmExecuteFailed)
            })
            .await
            .expect("spawn blocking join handle is infallible. qed.")?,
            false => tokio::task::spawn_blocking(move || {
                vm.execute_stateful(&elf, &input, &program_state, max_cycles)
                    .map_err(Error::ZkvmExecuteFailed)
            })
            .await
            .expect("spawn blocking join handle is infallible. qed.")?,
        };

        let result_with_metadata = abi_encode_result_with_metadata(
            job_id,
            program_input_hash,
            program_state_hash,
            max_cycles,
            &program_id,
            &raw_output,
        );
        let zkvm_operator_signature = self.sign_message(&result_with_metadata).await?;

        info!(
            job_id = ?job_id,
            vm_type = vm_type.as_str_name(),
            program_id = base64_program_id,
            raw_output = BASE64_STANDARD.encode(raw_output.as_slice()),
            "job complete"
        );

        Ok((result_with_metadata, zkvm_operator_signature))
    }

    /// Derives and returns program ID (verifying key) for the
    /// given program ELF.
    pub async fn create_elf(&self, elf: &[u8], vm_type: VmType) -> Result<Vec<u8>, Error> {
        let vm = self.vm(vm_type)?;

        let program_id = vm
            .derive_verifying_key(elf)
            .map_err(|e| Error::VerifyingKeyDerivationFailed(e.to_string()))?;

        let base64_program_id = BASE64_STANDARD.encode(program_id.as_slice());

        info!(vm_type = vm_type.as_str_name(), program_id = base64_program_id, "new elf program");

        Ok(program_id)
    }
}

sol! {
    /// The payload that gets signed to signify that the zkvm executor has faithfully
    /// executed the job. Also the result payload the job manager contract expects.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct ResultWithMetadata {
        /// Job ID.
        bytes32 job_id;
        /// Hash of input passed to zkVM program for this job.
        bytes32 program_input_hash;
        /// Hash of state passed to zkVM program for this job.
        bytes32 program_state_hash;
        /// Max cycles for the job.
        uint64 max_cycles;
        /// Program ID of program being executed.
        bytes program_id;
        /// Raw output (result) from zkVM program.
        bytes raw_output;
    }
}

/// Returns an ABI-encoded result with metadata. This ABI-encoded response will be
/// signed by the operator.
pub fn abi_encode_result_with_metadata(
    job_id: [u8; 32],
    program_input_hash: FixedBytes<32>,
    program_state_hash: FixedBytes<32>,
    max_cycles: u64,
    program_id: &[u8],
    raw_output: &[u8],
) -> Vec<u8> {
    ResultWithMetadata {
        job_id: job_id.into(),
        program_input_hash,
        program_state_hash,
        max_cycles,
        program_id: program_id.to_vec().into(),
        raw_output: raw_output.to_vec().into(),
    }
    .abi_encode()
}
