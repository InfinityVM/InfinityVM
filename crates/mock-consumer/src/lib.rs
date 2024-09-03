//! Utilities for setting testing with the `MockConsumer` contract.

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use contracts::mock_consumer::MockConsumer;
use db::tables::{get_job_id, Job, RequestType};
use proto::{JobStatus, JobStatusType};
use test_utils::{AnvilJobManager, MOCK_CONTRACT_MAX_CYCLES};
use zkvm_executor::service::abi_encode_result_with_metadata;

/// Output from [`anvil_with_mock_consumer`]
#[derive(Debug)]
pub struct AnvilMockConsumer {
    /// Address of the mock consumer contract
    pub mock_consumer: Address,
}

/// Deploy `MockConsumer` contracts to anvil instance
pub async fn anvil_with_mock_consumer(anvil_job_manager: &AnvilJobManager) -> AnvilMockConsumer {
    let AnvilJobManager { anvil, job_manager, .. } = anvil_job_manager;

    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
    let offchain_signer: PrivateKeySigner = anvil.keys()[5].clone().into();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let rpc_url = anvil.endpoint();
    let consumer_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    let initial_max_nonce = 0;
    let mock_consumer = MockConsumer::deploy(
        consumer_provider,
        *job_manager,
        offchain_signer.address(),
        initial_max_nonce,
    )
    .await
    .unwrap();
    let mock_consumer = *mock_consumer.address();

    AnvilMockConsumer { mock_consumer }
}

/// A mock address to use as input to the mock contract function calls
pub fn mock_contract_input_addr() -> Address {
    Address::default()
}

/// Mock raw output from the zkvm program for the mock consumer contract
pub fn mock_raw_output() -> Vec<u8> {
    (mock_contract_input_addr(), U256::default()).abi_encode()
}

/// Create a pending Job that has a signed result from the zkvm operator.
///
/// The result here will be decodable by the `MockConsumer` contract and have
/// a valid signature from the zkvm operator.
pub async fn mock_consumer_pending_job(
    nonce: u8,
    operator: PrivateKeySigner,
    mock_consumer: Address,
) -> Job {
    let bytes = vec![nonce; 32];
    let addr = mock_contract_input_addr();
    let raw_output = mock_raw_output();

    let job_id = get_job_id(nonce.into(), mock_consumer);
    let result_with_meta = abi_encode_result_with_metadata(
        job_id,
        &addr.abi_encode(),
        MOCK_CONTRACT_MAX_CYCLES,
        &bytes,
        &raw_output,
    )
    .unwrap();
    let operator_signature =
        operator.sign_message(&result_with_meta).await.unwrap().as_bytes().to_vec();

    Job {
        id: job_id,
        nonce: 1,
        max_cycles: MOCK_CONTRACT_MAX_CYCLES,
        program_id: bytes,
        input: addr.abi_encode(),
        program_state: vec![],
        request_type: RequestType::Onchain,
        result_with_metadata: result_with_meta,
        status: JobStatus {
            status: JobStatusType::Pending as i32,
            failure_reason: None,
            retries: 0,
        },
        consumer_address: mock_consumer.abi_encode(),
        zkvm_operator_signature: operator_signature,
    }
}
