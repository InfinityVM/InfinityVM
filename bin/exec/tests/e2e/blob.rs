//! E2E tests for blob transactions

use crate::utils::eth_payload_attributes;
use alloy_consensus::constants::MAINNET_GENESIS_HASH;
use alloy_genesis::Genesis;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types_engine::PayloadStatusEnum;
use ivm_exec::{
    config::{transaction::IvmTransactionAllowConfig, IvmConfig},
    IvmNode,
};
use reth_chainspec::{ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{
    node::NodeTestContext, transaction::TransactionTestContext, wallet::Wallet,
};
use reth_node_builder::{NodeBuilder, NodeHandle};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_tasks::TaskManager;
use reth_transaction_pool::TransactionPool;
use std::sync::Arc;

// This largely follows the blob test in reth introduced in https://github.com/paradigmxyz/reth/pull/7823.
//
// This test shows that we can construct payloads that reference blobs, the payload can be added
// to the fork, and then the payload can be removed (reorg-ed out) and the blob tx will be valid in
// the mempool
//
// RPCs implicitly tested:
// - debug_getRawTransaction with `node.api.envelope_by_hash`
// - eth_sendRawTransaction with `node.api.inject_tx`
//
// Test flow:
// - Build payload with normal tx
// - Build Payload with blob tx
// - Submit blob payload
// - Submit FCU with blob payload on top of genesis block
// - Submit normal payload
// - Submit FCU with normal payload, also on top of genesis block, creating a re-org
// - Expect the blob tx back in the pool with the sidecar
#[tokio::test]
async fn can_handle_blobs() -> eyre::Result<()> {
    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis =
        serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
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

    let wallets = Wallet::new(2).gen();
    let blob_wallet = wallets.first().unwrap();
    let second_wallet = wallets.last().unwrap();

    let mut config = IvmConfig::deny_all();
    let mut allow_config = IvmTransactionAllowConfig::deny_all();
    allow_config.add_sender(blob_wallet.address());
    allow_config.add_sender(second_wallet.address());
    config.set_fork(0, allow_config);

    let ivm_node_types = IvmNode::new(config);

    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await?;
    let mut node = NodeTestContext::new(node, eth_payload_attributes).await?;

    // Show that neither account has balance
    {
        let rpc = ProviderBuilder::new().on_http(node.rpc_url());
        // The account never gets created
        let get_account_err = rpc.get_account(blob_wallet.address()).await.unwrap_err().to_string();
        assert_eq!(
            &get_account_err,
            "deserialization error: invalid type: null, expected struct TrieAccount at line 1 column 4"
        );
        let get_account_err =
            rpc.get_account(second_wallet.address()).await.unwrap_err().to_string();
        assert_eq!(
            &get_account_err,
            "deserialization error: invalid type: null, expected struct TrieAccount at line 1 column 4"
        );
    }

    // Build payload with normal tx. This uses the IVM node payload builder component
    let raw_tx = TransactionTestContext::transfer_tx_bytes(1, second_wallet.clone()).await;
    let tx_hash = node.rpc.inject_tx(raw_tx).await?;
    let (payload, attributes) = node.new_payload().await?;

    // Clean the pool to ensure the blob tx below will be the only tx in the pool
    node.inner.pool().remove_transactions(vec![tx_hash]);

    // Build blob tx
    let blob_tx = TransactionTestContext::tx_with_blobs_bytes(1, blob_wallet.clone()).await?;
    // Inject blob tx to the pool. This should be the only tx in the pool
    let blob_tx_hash = node.rpc.inject_tx(blob_tx).await?;
    assert_eq!(node.inner.pool().pool_size().total, 1);

    // Fetch blob tx from rpc
    let envelope = node.rpc.envelope_by_hash(blob_tx_hash).await?;
    // Validate blob sidecar
    TransactionTestContext::validate_sidecar(envelope);

    // Build blob payload
    let (blob_payload, blob_attr) = node.new_payload().await?;

    // Submit the blob payload. At this point we have only submitted the blob payload
    let blob_block_hash =
        node.engine_api.submit_payload(blob_payload, blob_attr, PayloadStatusEnum::Valid).await?;
    // Blob tx is still in the pool
    assert_eq!(node.inner.pool().pool_size().total, 1);
    // Update FCU such that the tip of the chain is exclusively the blob payload
    let _ = node.engine_api.update_forkchoice(MAINNET_GENESIS_HASH, blob_block_hash).await;
    // The blob transaction has been removed from the pool
    assert_eq!(node.inner.pool().pool_size().total, 0);

    // Update the forkchoice to instead use the normal payload, re-orging out the blob payload
    node.engine_api
        .submit_payload(payload.clone(), attributes, PayloadStatusEnum::Valid)
        .await
        .unwrap();
    let _ = node.engine_api.update_forkchoice(MAINNET_GENESIS_HASH, payload.block().hash()).await;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Expect the blob tx to be back in the pool since its now in a fork and not strictly
    // canonical
    let envelope = node.rpc.envelope_by_hash(blob_tx_hash).await?;
    // make sure the sidecar is present
    TransactionTestContext::validate_sidecar(envelope);

    Ok(())
}
