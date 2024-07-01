//! gRPC service implementation.

use alloy::network::NetworkWallet;
use alloy::network::TxSigner;
use alloy::primitives::Signature;
use alloy::signers::Signer;
use core::slice::Iter;
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
use std::marker::PhantomData;
use std::marker::Send;
use zkvm::Zkvm;

use alloy::{
    network::EthereumWallet,
    // signers::local::LocalSigner,
    primitives::{address, U256},
};

///  The implementation of the ZkvmExecutor trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug)]
pub struct ZkvmExecutorService<T, S> {
    // TODO(zeke): we can make this generic over a signer
    // to add support for things like AWS, yubihsm etc
    wallet: S,
    _phantom: PhantomData<T>,
}

impl<T, S> ZkvmExecutorService<T, S>
where
    S: Signer<Signature> + Send + Sync + 'static,
{
    pub(crate) fn new(wallet: S) -> Self {
        Self {
            wallet,
            _phantom: PhantomData,
        }
    }

    fn zkvm_operator_eth_address() -> Vec<u8> {
        unimplemented!()
    }

    fn eth_sign() -> Vec<u8> {
        unimplemented!()
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
        let zkvm_operator_signature = self.wallet.sign_msg(signing_payload);
        let response = ExecuteResponse {
            inputs: Some(inputs),
            zkvm_operator_address: self.wallet.default_signer_address(),
            zkvm_operator_signature,
            raw_output,
        };

        Ok(response)
    }
}
