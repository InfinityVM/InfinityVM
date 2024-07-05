//! gRPC service implementation.

use alloy::{primitives::Signature, rlp::Encodable, signers::Signer};
use alloy_rlp::RlpEncodable;
use proto::{ExecuteRequest, ExecuteResponse, VerifiedInputs};
use std::marker::{PhantomData, Send};
use zkvm::Zkvm;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("signer error: {0}")]
    Signer(#[from] alloy::signers::Error),
}

///  The implementation of the `ZkvmExecutor` trait
/// TODO(zeke): do we want to make this generic over executor?
#[derive(Debug)]
pub(crate) struct ZkvmExecutorService<Z, S> {
    signer: S,
    chain_id: Option<u64>,
    _phantom: PhantomData<Z>,
}

impl<Z, S> ZkvmExecutorService<Z, S>
where
    S: Signer<Signature> + Send + Sync + 'static,
{
    pub(crate) const fn new(signer: S, chain_id: Option<u64>) -> Self {
        Self { signer, chain_id, _phantom: PhantomData }
    }

    /// Checksum address (hex string), as bytes.
    fn address_checksum_bytes(&self) -> Vec<u8> {
        self.signer.address().to_checksum(self.chain_id).as_bytes().to_vec()
    }

    /// Returns an RLP encoded signature over `eip191_hash_message(msg)`
    // TODO: (leaving this as an easy issue to fix)
    // https://linear.app/ethos-stake/issue/ETH-378/infinityrust-switch-executor-signature-encoding
    async fn sign_message(&self, msg: &[u8]) -> Result<Vec<u8>, Error> {
        self.signer
            .sign_message(msg)
            .await
            .map(|sig| {
                let mut out = Vec::with_capacity(sig.rlp_vrs_len());
                sig.encode(&mut out);
                out
            })
            .map_err(Into::into)
    }
}

#[tonic::async_trait]
impl<Z, S> proto::zkvm_executor_server::ZkvmExecutor for ZkvmExecutorService<Z, S>
where
    Z: Zkvm + Send + Sync + 'static,
    S: Signer<Signature> + Send + Sync + 'static,
{
    async fn execute(
        &self,
        request: tonic::Request<ExecuteRequest>,
    ) -> Result<tonic::Response<ExecuteResponse>, tonic::Status> {
        let msg = request.into_inner();
        let inputs = msg.inputs.expect("todo");

        if !Z::is_correct_verifying_key(&msg.program_elf, &inputs.program_verifying_key)
            .expect("todo")
        {
            return Err(tonic::Status::invalid_argument("bad verifying key"));
        }

        let raw_output = Z::execute(&msg.program_elf, &inputs.program_input, inputs.max_cycles)
            .map_err(|e| format!("zkvm execute error: {e:?}"))
            .map_err(tonic::Status::invalid_argument)?;

        let signing_payload = result_signing_payload(&inputs, &raw_output);

        let zkvm_operator_signature = self
            .sign_message(&signing_payload)
            .await
            .map_err(|e| format!("signing error: {e:?}"))
            .map_err(tonic::Status::internal)?;
        let response = ExecuteResponse {
            inputs: Some(inputs),
            zkvm_operator_address: self.address_checksum_bytes(),
            zkvm_operator_signature,
            raw_output,
        };

        Ok(tonic::Response::new(response))
    }
}

/// This gets RLP encoded to construct the singing payload.
#[derive(Debug, RlpEncodable)]
struct SingingPayload<'a> {
    program_verifying_key: &'a [u8],
    program_input: &'a [u8],
    max_cycles: u64,
    raw_output: &'a [u8],
}

fn result_signing_payload(i: &VerifiedInputs, raw_output: &[u8]) -> Vec<u8> {
    let result_len =
        i.program_input.len() + i.program_verifying_key.len() + raw_output.len() + 64 + 32;
    let mut out = Vec::with_capacity(result_len);
    let payload = SingingPayload {
        program_verifying_key: &i.program_verifying_key,
        program_input: &i.program_input,
        max_cycles: i.max_cycles,
        raw_output,
    };
    payload.encode(&mut out);

    out
}
