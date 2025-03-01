//! Utilities for setting testing with the `MockConsumer` contract.

use alloy::{
    network::EthereumWallet,
    primitives::{keccak256, Address, U256},
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use ivm_abi::{abi_encode_result_with_metadata, get_job_id};
use ivm_contracts::{
    mock_consumer::MockConsumer, proxy_admin::ProxyAdmin,
    transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use ivm_db::tables::{Job, RequestType};
use ivm_proto::{JobStatus, JobStatusType};
use ivm_test_utils::{get_signers, IvmExecJobManager};

/// Max cycles that the `MockContract` calls create job with.
pub const MOCK_CONSUMER_MAX_CYCLES: u64 = 1_000_000;

/// Output from [`ivm_exec_with_mock_consumer`]
#[derive(Debug)]
pub struct IvmExecMockConsumer {
    /// Address of the mock consumer contract
    pub mock_consumer: Address,
    /// Offchain signer for mock consumer.
    pub mock_consumer_signer: PrivateKeySigner,
}

/// Deploy `MockConsumer` contracts to ivm-exec instance
pub async fn ivm_exec_with_mock_consumer(
    ivm_job_manager: &IvmExecJobManager,
) -> IvmExecMockConsumer {
    let signers = get_signers(6);
    let IvmExecJobManager { ivm_exec, job_manager, .. } = ivm_job_manager;

    let consumer_owner: PrivateKeySigner = signers[4].clone();
    let offchain_signer: PrivateKeySigner = signers[5].clone();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let consumer_provider = ProviderBuilder::new()
        .wallet(consumer_owner_wallet)
        .on_http(ivm_exec.endpoint().parse().unwrap());

    // Deploy mock consumer implementation
    let mock_consumer_impl = MockConsumer::deploy(consumer_provider.clone()).await.unwrap();

    // Deploy proxy admin
    let proxy_admin = ProxyAdmin::deploy(consumer_provider.clone()).await.unwrap();

    let initializer = mock_consumer_impl.initialize_0(
        consumer_owner.address(),
        *job_manager,
        0,
        offchain_signer.address(),
    );
    let initializer_calldata = initializer.calldata();

    // Deploy a proxy contract for MockConsumer
    let mock_consumer = TransparentUpgradeableProxy::deploy(
        &consumer_provider,
        *mock_consumer_impl.address(),
        *proxy_admin.address(),
        initializer_calldata.clone(),
    )
    .await
    .unwrap();

    IvmExecMockConsumer {
        mock_consumer: *mock_consumer.address(),
        mock_consumer_signer: offchain_signer,
    }
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
        keccak256(addr.abi_encode()),
        MOCK_CONSUMER_MAX_CYCLES,
        &bytes,
        &raw_output,
    );
    let operator_signature =
        operator.sign_message(&result_with_meta).await.unwrap().as_bytes().to_vec();

    Job {
        id: job_id,
        nonce: 1,
        max_cycles: MOCK_CONSUMER_MAX_CYCLES,
        program_id: bytes,
        onchain_input: addr.abi_encode(),
        offchain_input: vec![],
        request_type: RequestType::Onchain,
        result_with_metadata: result_with_meta,
        status: JobStatus {
            status: JobStatusType::Pending as i32,
            failure_reason: None,
            retries: 0,
        },
        consumer_address: mock_consumer.abi_encode(),
        zkvm_operator_signature: operator_signature,
        relay_tx_hash: vec![],
        blobs_sidecar: None,
        relay_strategy: ivm_proto::RelayStrategy::Unordered,
    }
}
