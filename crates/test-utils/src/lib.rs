//! Utilities for setting up tests.

use std::net::TcpListener;

use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, U256},
    providers::{ext::AnvilApi, ProviderBuilder},
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use contracts::{
    job_manager::JobManager, mock_consumer::MockConsumer,
    transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use rand::Rng;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;
use zkvm_executor::service::abi_encode_result_with_metadata;

/// Max cycles that the `MockContract` calls create job with.
pub const MOCK_CONTRACT_MAX_CYCLES: u64 = 1_000_000;

/// Localhost IP address
pub const LOCALHOST: &str = "127.0.0.1";

/// Initialize a tracing subscriber for tests. Use `RUSTLOG` to set the filter level.
pub fn test_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init()
        .unwrap();
}

/// Find a free port on localhost.
pub fn get_localhost_port() -> u16 {
    let mut rng = rand::thread_rng();

    for _ in 0..64 {
        let port = rng.gen_range(49152..65535);
        if TcpListener::bind((LOCALHOST, port)).is_ok() {
            return port;
        }
    }

    panic!("no port found after 64 attempts");
}

/// Sleep until the given port is bound.
pub async fn sleep_until_bound(port: u16) {
    for _ in 0..16 {
        if TcpListener::bind((LOCALHOST, port)).is_err() {
            return;
        }

        sleep(Duration::from_secs(1)).await;
    }

    panic!("localhost:{port} was not successfully bound");
}

/// Output from [`anvil_with_contracts`]
#[derive(Debug)]
pub struct TestAnvil {
    /// Anvil instance
    pub anvil: AnvilInstance,
    /// Address of the job manager contract
    pub job_manager: Address,
    /// Relayer private key
    pub relayer: PrivateKeySigner,
    /// Coprocessor operator private key
    pub coprocessor_operator: PrivateKeySigner,
    /// Address of the mock consumer contract
    pub mock_consumer: Address,
}

/// Setup an anvil instance with job manager contracts.
pub async fn anvil_with_contracts() -> TestAnvil {
    // Ensure the anvil instance will not collide with anything already running on the OS
    let port = get_localhost_port();
    // Set block time to 0.01 seconds - I WANNA GO FAST MOM
    let anvil = Anvil::new().block_time_f64(0.01).port(port).try_spawn().unwrap();

    let initial_owner: PrivateKeySigner = anvil.keys()[0].clone().into();
    let relayer: PrivateKeySigner = anvil.keys()[1].clone().into();
    let coprocessor_operator: PrivateKeySigner = anvil.keys()[2].clone().into();
    let proxy_admin: PrivateKeySigner = anvil.keys()[3].clone().into();
    let consumer_owner: PrivateKeySigner = anvil.keys()[4].clone().into();
    let offchain_signer: PrivateKeySigner = anvil.keys()[5].clone().into();

    let initial_owner_wallet = EthereumWallet::from(initial_owner.clone());
    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let rpc_url = anvil.endpoint();
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(initial_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

    // configure anvil over rpc
    // note: we can also configure the url in the future
    provider.anvil_set_logging(true).await.unwrap();
    provider.anvil_set_auto_mine(true).await.unwrap();

    // Deploy the JobManager implementation contract
    let job_manager_implementation = JobManager::deploy(&provider).await.unwrap();

    // initializeJobManager will be called later when we deploy the proxy
    let initializer = job_manager_implementation.initializeJobManager(
        initial_owner.address(),
        relayer.address(),
        coprocessor_operator.address(),
    );
    let initializer_calldata = initializer.calldata();

    // Deploy a proxy contract for JobManager
    let proxy = TransparentUpgradeableProxy::deploy(
        &provider,
        *job_manager_implementation.address(),
        proxy_admin.address(),
        initializer_calldata.clone(),
    )
    .await
    .unwrap();

    let job_manager = *proxy.address();

    let consumer_provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(consumer_owner_wallet)
        .on_http(rpc_url.parse().unwrap());

    // Deploy the mock consumer contract. This can take jobs and accept results.
    let mock_consumer =
        MockConsumer::deploy(consumer_provider, job_manager, offchain_signer.address())
            .await
            .unwrap();
    let mock_consumer = *mock_consumer.address();

    TestAnvil { anvil, job_manager, relayer, coprocessor_operator, mock_consumer }
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
    i: u8,
    operator: PrivateKeySigner,
    mock_consumer: Address,
) -> proto::Job {
    let bytes = vec![i; 32];
    let addr = mock_contract_input_addr();
    let raw_output = mock_raw_output();

    let inputs = proto::JobInputs {
        job_id: i as u32,
        max_cycles: MOCK_CONTRACT_MAX_CYCLES,
        program_verifying_key: bytes.clone(),
        program_input: addr.abi_encode(),
        vm_type: 1,
        program_elf: bytes.clone(),
    };

    let result_with_meta = abi_encode_result_with_metadata(&inputs, &raw_output);
    let operator_signature =
        operator.sign_message(&result_with_meta).await.unwrap().as_bytes().to_vec();

    let job = proto::Job {
        id: inputs.job_id,
        max_cycles: inputs.max_cycles,
        program_verifying_key: inputs.program_verifying_key,
        input: inputs.program_input,
        result: result_with_meta,
        status: proto::JobStatus::Pending as i32,
        contract_address: mock_consumer.abi_encode(),
        zkvm_operator_signature: operator_signature,
        zkvm_operator_address: operator.address().to_checksum(None).as_bytes().to_vec(),
    };

    job
}
