use std::sync::Arc;

use alloy_genesis::Genesis;
use ivm_exec::{pool::validator::IvmTransactionAllowConfig, IvmNode};
use ivm_test_utils::wallet::Wallet;
use reth::args::RpcServerArgs;
use reth_chainspec::{ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{node::NodeTestContext, transaction::TransactionTestContext};
use reth_node_builder::{NodeBuilder, NodeConfig, NodeHandle};
use reth_tasks::TaskManager;
use crate::utils::eth_payload_attributes;

#[tokio::test]
async fn denies_non_allowed_senders() -> eyre::Result<()> {
    ivm_test_utils::test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis = serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis)
            .build(),
    );
    let node_config = NodeConfig::test()
        .with_chain(chain_spec)
        .with_unused_ports()
        .with_rpc(RpcServerArgs::default().with_unused_ports().with_http());

    // TODO: use non-funded addresses
    let wallets = Wallet::new(2).gen();
    let blob_wallet = wallets.first().unwrap();
    let alice_wallet = wallets.last().unwrap();

    // Everyone is denied
    let mut txn_allow = IvmTransactionAllowConfig::deny_all();
    let ivm_node_types = IvmNode::new(txn_allow);

    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await?;
    let mut node = NodeTestContext::new(node, eth_payload_attributes).await?;

    let transfer_tx = TransactionTestContext::transfer_tx_bytes(1, alice_wallet.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap_err();

    // node.rpc.
}