//! gRPC service implementation.

use alloy::{
    primitives::Signature,
    signers::{Signer, SignerSync},
};
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
use std::marker::{PhantomData, Send};
use zkvm::Zkvm;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
}

///  The implementation of the ZkvmExecutor trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug)]
pub struct ZkvmExecutorService<T, S> {
    // TODO(zeke): we can make this generic over a signer
    // to add support for things like AWS, yubihsm etc
    wallet: S,
    chain_id: Option<u64>,
    _phantom: PhantomData<T>,
}

impl<T, S> ZkvmExecutorService<T, S>
where
    S: Signer<Signature> + SignerSync + Send + Sync + 'static,
{
    pub(crate) fn new(wallet: S, chain_id: Option<u64>) -> Self {
        Self { wallet, chain_id, _phantom: PhantomData }
    }

    fn checksum_address_bytes(&self) -> Vec<u8> {
        self.wallet.address().to_checksum(self.chain_id).as_bytes().to_vec()
    }

    fn sign_message(&self, msg: &[u8]) -> Vec<u8> {
        self.wallet.sign_message_sync(msg).map(|s| s.to_bytes())
    }
}

fn result_signing_payload(i: &VerifiedInputs, raw_output: &[u8]) -> Vec<u8> {
    // TODO(zeke): should we just create a payload struct and RLP encode
    // that instead? This seems like a finicky encoding strategy

    i.program_verifying_key
        .iter()
        .chain(i.program_input.iter())
        .chain(i.max_cycles.to_be_bytes().iter())
        .chain(raw_output)
        .copied()
        .collect()
}

#[tonic::async_trait]
impl<T, S> proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService<T, S>
where
    T: Zkvm + Send + Sync + 'static,
    S: Signer<Signature> + Send + Sync + 'static,
{
    async fn execute(
        &self,
        request: tonic::Request<ExecuteRequest>,
    ) -> Result<tonic::Response<ExecuteResponse>, tonic::Status> {
        let msg = request.into_inner();
        let inputs = msg.inputs.expect("todo");

        if !T::is_correct_verifying_key(&msg.program_elf, &inputs.program_verifying_key)
            .expect("todo")
        {
            return Err(tonic::Status::invalid_argument("bad verifying key"));
        }

        let raw_output = T::execute(
            &msg.program_elf,
            &inputs.program_input,
            // TODO(zeke) make this safe
            inputs.max_cycles as u64,
        )
        .map_err(|e| format!("zkvm execute error: {e:?}"))
        .map_err(|e| tonic::Status::invalid_argument(e))?;

        let signing_payload = result_signing_payload(&inputs, &raw_output);

        // TODO(zeke): make sure this is getting keccak hash
        let zkvm_operator_signature = self.sign_message(&signing_payload);
        let response = ExecuteResponse {
            inputs: Some(inputs),
            zkvm_operator_address: self.checksum_address_bytes(),
            zkvm_operator_signature,
            raw_output,
        };

        Ok(response)
    }
}
