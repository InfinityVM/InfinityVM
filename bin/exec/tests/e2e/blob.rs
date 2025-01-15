//! E2E tests for blob transactions

use crate::utils::eth_payload_attributes;
use alloy_consensus::constants::MAINNET_GENESIS_HASH;
use alloy_genesis::Genesis;
use alloy_rpc_types_engine::PayloadStatusEnum;
use ivm_exec::{pool::validator::IvmTransactionAllowConfig, IvmNode};
use reth_chainspec::{ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{
    node::NodeTestContext, transaction::TransactionTestContext, wallet::Wallet,
};
use reth_node_builder::{NodeBuilder, NodeHandle};
use reth_node_core::{args::RpcServerArgs, node_config::NodeConfig};
use reth_tasks::TaskManager;
use reth_transaction_pool::TransactionPool;
use std::{collections::HashSet, sync::Arc};

// TODO
// - check that only 6 blobs allowed

// This test shows that we can construct payloads that reference blobs, the payload can be added
// to the fork, and then the payload can be removed and the blob tx will be valid in the mempool
//
// RPCs implicitly tested:
// - debug_getRawTransaction with `node.api.envelope_by_hash`
// - eth_sendRawTransaction with `node.api.envelope_by_hash`
#[tokio::test]
async fn can_handle_blobs() -> eyre::Result<()> {
    // ivm_test_utils::test_tracing();

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

    // TODO: use non-funded addresses
    let wallets = Wallet::new(2).gen();
    let blob_wallet = wallets.first().unwrap();
    let second_wallet = wallets.last().unwrap();

    let mut txn_allow = IvmTransactionAllowConfig::deny_all();
    txn_allow.set_sender(HashSet::from([blob_wallet.address(), second_wallet.address()]));

    let ivm_node_types = IvmNode::new(txn_allow);

    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await?;
    let mut node = NodeTestContext::new(node, eth_payload_attributes).await?;

    // // inject normal tx
    let raw_tx = TransactionTestContext::transfer_tx_bytes(1, second_wallet.clone()).await;
    let tx_hash = node.rpc.inject_tx(raw_tx).await?;
    // // build payload with normal tx. This uses the node payload builder component
    let (payload, attributes) = node.new_payload().await?;

    // clean the pool
    node.inner.pool().remove_transactions(vec![tx_hash]);

    // build blob tx
    let blob_tx = TransactionTestContext::tx_with_blobs_bytes(1, blob_wallet.clone()).await?;

    // inject blob tx to the pool. This should be the only tx in the pool
    let blob_tx_hash = node.rpc.inject_tx(blob_tx).await?;
    assert_eq!(node.inner.pool().pool_size().total, 1);

    // fetch it from rpc
    let envelope = node.rpc.envelope_by_hash(blob_tx_hash).await?;
    // validate sidecar
    TransactionTestContext::validate_sidecar(envelope);

    // build a payload
    let (blob_payload, blob_attr) = node.new_payload().await?;

    // submit the blob payload
    let blob_block_hash =
        node.engine_api.submit_payload(blob_payload, blob_attr, PayloadStatusEnum::Valid).await?;

    let (_, _) = tokio::join!(
        // send fcu with blob hash
        node.engine_api.update_forkchoice(MAINNET_GENESIS_HASH, blob_block_hash),
        // send fcu with normal hash
        node.engine_api.update_forkchoice(MAINNET_GENESIS_HASH, payload.block().hash())
    );

    assert_eq!(node.inner.pool().pool_size().total, 0);

    // submit normal payload
    node.engine_api.submit_payload(payload, attributes, PayloadStatusEnum::Valid).await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // expects the blob tx to be back in the pool
    let envelope = node.rpc.envelope_by_hash(blob_tx_hash).await?;
    // make sure the sidecar is present
    TransactionTestContext::validate_sidecar(envelope);

    Ok(())
}
