use std::sync::Arc;

use alloy_genesis::Genesis;
use alloy_primitives::U256;
use ivm_exec::{pool::validator::IvmTransactionAllowConfig, IvmNode};
use reth::args::RpcServerArgs;
use reth_chainspec::{ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{node::NodeTestContext, transaction::TransactionTestContext, wallet::Wallet};
use reth_node_builder::{NodeBuilder, NodeConfig, NodeHandle};
use reth_tasks::TaskManager;
use crate::utils::eth_payload_attributes;
use alloy_provider::{Provider, ProviderBuilder};

#[tokio::test]
async fn denies_non_allowed_senders() -> eyre::Result<()> {
    // ivm_test_utils::test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis = serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis)
            .cancun_activated()
            .build(),
    );
    let node_config = NodeConfig::test()
        .with_chain(chain_spec)
        .with_unused_ports()
        .with_rpc(RpcServerArgs::default().with_unused_ports().with_http());

    // TODO: use non-funded addresses
    let wallets = Wallet::new(2).gen();
    let alice_wallet = wallets.last().unwrap();

    // Everyone is denied
    let ivm_node_types = IvmNode::new(IvmTransactionAllowConfig::deny_all());

    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await?;
    let node = NodeTestContext::new(node, eth_payload_attributes).await?;

    let rpc = ProviderBuilder::new()
        .on_http(node.rpc_url());

    let alice_balance = rpc.get_account(alice_wallet.address()).await.unwrap();
    assert_eq!(alice_balance.balance, U256::from(0));
    assert_eq!(alice_balance.nonce, 0);

    let transfer_tx = TransactionTestContext::transfer_tx_bytes(1, alice_wallet.clone()).await;
    // this calls eth send_raw_transaction
    let transfer_error = node.rpc.inject_tx(transfer_tx).await.unwrap_err();
    dbg!(transfer_error);
    // assert_eq!(
    //     transfer_error,

    // );


    let alice_balance = rpc.get_account(alice_wallet.address()).await.unwrap();
    assert_eq!(alice_balance.balance, U256::from(0));
    assert_eq!(alice_balance.nonce, 0);

    Ok(())
}