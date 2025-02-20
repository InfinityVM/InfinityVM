//! High level test utilities specifically for the CLOB.

use crate::mock_erc20::MockErc20;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::{ProviderBuilder, WalletProvider},
    signers::local::PrivateKeySigner,
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::{api::Request, tick, BorshKeccak256, ClobState};
use ivm_contracts::{
    proxy_admin::ProxyAdmin, transparent_upgradeable_proxy::TransparentUpgradeableProxy,
};
use ivm_test_utils::{get_signers, AnvilJobManager, IvmExecJobManager};

/// `E2EMockERC20.sol` bindings
pub mod mock_erc20 {
    #![allow(clippy::all, missing_docs)]
    alloy::sol! {
        /// `E2EMockERC20`
        #[sol(rpc)]
        MockErc20,
        "../../../contracts/out/E2EMockERC20.sol/E2EMockERC20.json"
    }
}

/// Output form [`anvil_with_clob_consumer`]
/// TODO: rename 
#[derive(Debug)]
pub struct AnvilClob {
    /// Offchain signer for clob.
    pub clob_signer: PrivateKeySigner,
    /// Address of the clob consumer contract
    pub clob_consumer: Address,
    /// Address of quote asset erc20
    pub quote_erc20: Address,
    /// Address of base asset erc20
    pub base_erc20: Address,
}

/// Deploy `ClobConsumer` to anvil instance.
/// TODO: delete
pub async fn anvil_with_clob_consumer(anvil: &AnvilJobManager) -> AnvilClob {
    let AnvilJobManager { anvil, job_manager, .. } = anvil;
    clob_consumer_deploy(anvil.endpoint(), job_manager).await
}

/// Deploy `ClobConsumer` to ivm-exec instance.
pub async fn  ivm_exec_with_clob_consumer(ivm_exec: &IvmExecJobManager) -> AnvilClob {
    let IvmExecJobManager { ivm_exec, job_manager, .. } = ivm_exec;
    clob_consumer_deploy(ivm_exec.endpoint(), job_manager).await
}

/// Deploy clob consumer contracts.
pub async fn clob_consumer_deploy(rpc_url: String, job_manager: &Address) -> AnvilClob {
    let signers = get_signers(6);

    let consumer_owner: PrivateKeySigner = signers[4].clone();
    let clob_signer: PrivateKeySigner = signers[5].clone();

    let consumer_owner_wallet = EthereumWallet::from(consumer_owner.clone());

    let provider = ProviderBuilder::new()
        .wallet(consumer_owner_wallet.clone())
        .on_http(rpc_url.parse().unwrap());

    // Deploy base & quote erc20s
    let base_name = "base".to_string();
    let base_symbol = "BASE".to_string();
    let base_erc20 =
        *mock_erc20::MockErc20::deploy(&provider, base_name, base_symbol).await.unwrap().address();

    let quote_name = "quote".to_string();
    let quote_symbol = "QUOTE".to_string();
    let quote_erc20 = *mock_erc20::MockErc20::deploy(&provider, quote_name, quote_symbol)
        .await
        .unwrap()
        .address();

    let clob_state0 = ClobState::default();
    // TODO (Maanav): change this from state hash to state root
    // [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
    let init_state_hash: [u8; 32] = clob_state0.borsh_keccak256().into();

    // Deploy clob consumer implementation
    let clob_consumer_impl = ClobConsumer::deploy(provider.clone()).await.unwrap();

    // Deploy proxy admin
    let proxy_admin = ProxyAdmin::deploy(provider.clone()).await.unwrap();

    let initializer = clob_consumer_impl.initialize_3(
        consumer_owner.address(),
        *job_manager,
        0,
        init_state_hash.into(),
        clob_signer.address(),
        base_erc20,
        quote_erc20,
    );
    let initializer_calldata = initializer.calldata();

    // Deploy a proxy contract for ClobConsumer
    let clob_consumer = TransparentUpgradeableProxy::deploy(
        &provider,
        *clob_consumer_impl.address(),
        *proxy_admin.address(),
        initializer_calldata.clone(),
    )
    .await
    .unwrap();

    AnvilClob { clob_signer, clob_consumer: *clob_consumer.address(), quote_erc20, base_erc20 }
}

/// Mint erc20s and approve transfers to the first `count` anvil auto seeded accounts.
pub async fn mint_and_approve(clob: &AnvilClob, http_endpoint: String, count: usize) {
    let signers: Vec<_> = get_signers(count).into_iter().map(EthereumWallet::from).collect();

    for signer in signers {
        let provider =
            ProviderBuilder::new().wallet(signer.clone()).on_http(http_endpoint.parse().unwrap());

        let quote_erc20 = MockErc20::new(clob.quote_erc20, &provider);

        let amount = U256::try_from(u64::MAX).unwrap();
        let call_builder = quote_erc20.mint(provider.default_signer_address(), amount);
        let r1 = call_builder.send().await.unwrap().get_receipt();

        let call_builder = quote_erc20.approve(clob.clob_consumer, amount);
        let r2 = call_builder.send().await.unwrap().get_receipt();

        let base_erc20 = MockErc20::new(clob.base_erc20, &provider);
        let call_builder = base_erc20.mint(provider.default_signer_address(), amount);
        let r3 = call_builder.send().await.unwrap().get_receipt();

        let call_builder = base_erc20.approve(clob.clob_consumer, amount);
        let r4 = call_builder.send().await.unwrap().get_receipt();

        tokio::try_join!(r1, r2, r3, r4).unwrap();
    }
}

/// Returns the next state given a list of transactions.
pub fn next_state(txns: Vec<Request>, init_state: ClobState) -> ClobState {
    let mut next_clob_state = init_state;
    for tx in txns.iter().cloned() {
        (_, next_clob_state, _) = tick(tx, next_clob_state);
    }

    next_clob_state
}
