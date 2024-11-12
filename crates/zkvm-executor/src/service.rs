//! zkVM execution logic.

use alloy::{
    consensus::BlobTransactionSidecar,
    hex,
    primitives::{keccak256, Address, Signature},
    signers::Signer,
};
use eip4844::{SidecarBuilder, SimpleCoder};
use ivm_abi::{abi_encode_offchain_result_with_metadata, abi_encode_result_with_metadata};
use ivm_proto::VmType;
use ivm_zkvm::Zkvm;
use std::marker::Send;
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
    ZkvmExecuteFailed(#[from] ivm_zkvm::Error),
    /// c-kzg logic had an error.
    #[error("c-kzg: {0}")]
    Kzg(#[from] eip4844::CKzgError),
}

/// The implementation of the `ZkvmExecutor` trait
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
            VmType::Risc0 => Box::new(ivm_zkvm::Risc0),
            VmType::Sp1 => Box::new(ivm_zkvm::Sp1),
            // VmType::Sp1Executor => Box::new(zkvm::Sp1Executor),
        };

        Ok(vm)
    }

    /// Executes a program on the given inputs, and returns signed output.
    /// Returns (`result_with_metadata`, `zkvm_operator_signature`)
    ///
    /// WARNING: this does not check the verifying key of the program. It is up to the caller to
    /// ensure that they trust the ELF has the correct verifying key onchain before committing to
    /// the result. Otherwise they risk being slashed because any proof will not verify.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_onchain_job(
        &self,
        job_id: [u8; 32],
        max_cycles: u64,
        program_id: Vec<u8>,
        onchain_input: Vec<u8>,
        elf: Vec<u8>,
        vm_type: VmType,
    ) -> Result<(Vec<u8>, Vec<u8>), Error> {
        let vm = self.vm(vm_type)?;

        let onchain_input_hash = keccak256(&onchain_input);
        let raw_output = tokio::task::spawn_blocking(move || {
            vm.execute(&elf, &onchain_input, &[], max_cycles).map_err(Error::ZkvmExecuteFailed)
        })
        .await
        .expect("spawn blocking join handle is infallible. qed.")?;

        let result_with_metadata = abi_encode_result_with_metadata(
            job_id,
            onchain_input_hash,
            max_cycles,
            &program_id,
            &raw_output,
        );
        let zkvm_operator_signature = self.sign_message(&result_with_metadata).await?;

        Ok((result_with_metadata, zkvm_operator_signature))
    }

    /// Executes an offchain job on the given inputs, and returns signed
    /// output. Returns (`offchain_result_with_metadata`, `zkvm_operator_signature`,
    /// `maybe_blobs_sidecar`). If the offchain input is empty, the blobs sidecar will be None.
    ///
    /// WARNING: this does not check the verifying key of the program. It is up to the caller to
    /// ensure that they trust the ELF has the correct verifying key onchain before committing to
    /// the result. Otherwise they risk being slashed because any proof will not verify.
    #[allow(clippy::too_many_arguments)]
    pub async fn execute_offchain_job(
        &self,
        job_id: [u8; 32],
        max_cycles: u64,
        program_id: Vec<u8>,
        onchain_input: Vec<u8>,
        offchain_input: Vec<u8>,
        elf: Vec<u8>,
        vm_type: VmType,
    ) -> Result<(Vec<u8>, Vec<u8>, Option<BlobTransactionSidecar>), Error> {
        let vm = self.vm(vm_type)?;

        let onchain_input_hash = keccak256(&onchain_input);
        let offchain_input_hash = keccak256(&offchain_input);
        let (raw_output, sidecar) = tokio::task::spawn_blocking(move || {
            let raw_output = vm
                .execute(&elf, &onchain_input, &offchain_input, max_cycles)
                .map_err(Error::ZkvmExecuteFailed)?;

            // We also build blob sidecars (with proofs) in this blocking task since
            // it is probably too slow for a standard tokio task
            let sidecar = if !offchain_input.is_empty() {
                let sidecar_builder: SidecarBuilder<SimpleCoder> =
                    std::iter::once(offchain_input).collect();
                Some(sidecar_builder.build()?)
            } else {
                None
            };

            Ok::<(Vec<u8>, Option<BlobTransactionSidecar>), Error>((raw_output, sidecar))
        })
        .await
        .expect("spawn blocking join handle is infallible. qed.")?;

        let versioned_hashes =
            sidecar.as_ref().map(|s| s.versioned_hashes().collect()).unwrap_or_default();
        let offchain_result_with_metadata = abi_encode_offchain_result_with_metadata(
            job_id,
            onchain_input_hash,
            offchain_input_hash,
            max_cycles,
            &program_id,
            &raw_output,
            versioned_hashes,
        );
        let zkvm_operator_signature = self.sign_message(&offchain_result_with_metadata).await?;

        Ok((offchain_result_with_metadata, zkvm_operator_signature, sidecar))
    }

    /// Derives and returns program ID (verifying key) for the
    /// given program ELF.
    pub async fn create_elf(&self, elf: &[u8], vm_type: VmType) -> Result<Vec<u8>, Error> {
        let vm = self.vm(vm_type)?;

        let program_id = vm
            .derive_verifying_key(elf)
            .map_err(|e| Error::VerifyingKeyDerivationFailed(e.to_string()))?;

        info!(
            vm_type = vm_type.as_str_name(),
            program_id = hex::encode(&program_id),
            "new elf program"
        );

        Ok(program_id)
    }
}
