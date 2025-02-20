#![allow(clippy::large_stack_frames)]

use crate::utils::{assert_unsupported_tx, eth_payload_attributes, signed_bytes, transfer_bytes};
use alloy_genesis::Genesis;
use alloy_network::EthereumWallet;
use alloy_primitives::{address, U256};
use alloy_provider::{Provider, ProviderBuilder};
use ivm_exec::{
    config::{transaction::IvmTransactionAllowConfig, IvmConfig},
    IvmNode,
};
use reth::args::RpcServerArgs;
use reth_chainspec::{ChainSpecBuilder, MAINNET};
use reth_e2e_test_utils::{
    node::NodeTestContext, transaction::TransactionTestContext, wallet::Wallet,
};
use reth_node_builder::{NodeBuilder, NodeConfig, NodeHandle};
use reth_tasks::TaskManager;
use reth_transaction_pool::TransactionPool;
use std::sync::Arc;

alloy_sol_types::sol! {
    #[sol(rpc, bytecode = "6080604052348015600f57600080fd5b5060405160db38038060db833981016040819052602a91607a565b60005b818110156074576040805143602082015290810182905260009060600160408051601f19818403018152919052805160209091012080555080606d816092565b915050602d565b505060b8565b600060208284031215608b57600080fd5b5051919050565b60006001820160b157634e487b7160e01b600052601160045260246000fd5b5060010190565b60168060c56000396000f3fe6080604052600080fdfea164736f6c6343000810000a")]
    contract GasWaster {
        constructor(uint256 iterations) {
            for (uint256 i = 0; i < iterations; i++) {
                bytes32 slot = keccak256(abi.encode(block.number, i));
                assembly {
                    sstore(slot, slot)
                }
            }
        }
    }
}

#[tokio::test]
async fn denies_non_allowed_senders() -> eyre::Result<()> {
    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis =
        serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis.clone())
            .cancun_activated()
            .build(),
    );
    let node_config = NodeConfig::test()
        .with_chain(chain_spec)
        .with_unused_ports()
        .with_rpc(RpcServerArgs::default().with_unused_ports().with_http());

    let wallets = Wallet::new(3).gen();
    let wallet_0 = wallets[0].clone();
    let wallet_1 = wallets[1].clone();

    // Everyone is denied
    let ivm_node_types = IvmNode::new(IvmConfig::deny_all());

    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await?;
    let mut node = NodeTestContext::new(node, eth_payload_attributes).await?;

    let rpc = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(EthereumWallet::new(wallet_1.clone()))
        .on_http(node.rpc_url());

    // Special block 0 edge case where all transactions are accepted
    let transfer_tx = transfer_bytes(0, None, wallet_0.clone()).await;
    node.rpc.inject_tx(transfer_tx).await?;
    node.advance_block().await?;

    let transfer_tx2 = transfer_bytes(1, None, wallet_0.clone()).await;
    let transfer_error = node.rpc.inject_tx(transfer_tx2).await.unwrap_err();
    assert_unsupported_tx(transfer_error);

    let blob_tx = TransactionTestContext::tx_with_blobs_bytes(1, wallet_1.clone()).await?;
    let blob_error = node.rpc.inject_tx(blob_tx).await.unwrap_err();
    assert_unsupported_tx(blob_error);

    let deploy_error =
        GasWaster::deploy_builder(&rpc, U256::from(500)).send().await.unwrap_err().to_string();
    assert!(&deploy_error.contains("-32603"));
    assert!(&deploy_error.contains("non-allowed transaction sender and recipient"));

    // The account never gets created
    let get_account_err = rpc.get_account(wallet_1.address()).await.unwrap_err().to_string();
    assert_eq!(
        &get_account_err,
        "deserialization error: invalid type: null, expected struct TrieAccount at line 1 column 4"
    );

    // And sanity check that pre-alloc'ed accounts can be queried
    let alloc_account = address!("0x7e480b98e3710753ffb23f67bd35391d5a6b1e9e");
    assert!(genesis.alloc.contains_key(&alloc_account));
    let account = rpc.get_account(alloc_account).await?;
    assert_eq!(account.nonce, 0);
    assert_eq!(account.balance, U256::from(0x12345));

    Ok(())
}

#[tokio::test]
async fn allow_config_is_fork_aware() {
    ivm_test_utils::test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis =
        serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis.clone())
            .cancun_activated()
            .build(),
    );
    let node_config = NodeConfig::test()
        .with_chain(chain_spec)
        .with_unused_ports()
        .with_rpc(RpcServerArgs::default().with_unused_ports().with_http());

    let wallets = Wallet::new(6).gen();
    let wallet_0 = wallets[0].clone();
    let wallet_1 = wallets[1].clone();
    let wallet_2 = wallets[2].clone();
    let wallet_3 = wallets[3].clone();
    let wallet_4 = wallets[4].clone();
    let wallet_5 = wallets[5].clone();

    // Deny all expect for the first block, which has timestamp 0.
    let mut config = IvmConfig::deny_all();

    // Only allow transactions from wallet 0
    let mut fork1 = IvmTransactionAllowConfig::deny_all();
    fork1.add_sender(wallet_0.address());

    // Only allow transactions going to wallet 1
    let mut fork2 = IvmTransactionAllowConfig::deny_all();
    fork2.add_to(wallet_1.address());

    // Allow all transactions
    let mut fork3 = IvmTransactionAllowConfig::deny_all();
    fork3.set_all(true);

    // From https://github.com/InfinityVM/reth/blob/main/crates/e2e-test-utils/src/payload.rs#L13
    let test_context_start_timestamp = 1710338135;

    // Setup the forks with 3 second offsets
    config.set_fork(test_context_start_timestamp, fork1);
    config.set_fork(test_context_start_timestamp + 3, fork2);
    config.set_fork(test_context_start_timestamp + (2 * 3), fork3);

    let ivm_node_types = IvmNode::new(config);
    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await
        .unwrap();
    let mut node = NodeTestContext::new(node, eth_payload_attributes).await.unwrap();

    // The very first timestamp will be zero, which triggers a special case of allowing any tx
    let transfer_tx_from_random = transfer_bytes(0, None, wallet_5.clone()).await;
    node.rpc.inject_tx(transfer_tx_from_random).await.unwrap();

    // Every call to new payload will increment the timestamp
    // https://github.com/InfinityVM/reth/blob/main/crates/e2e-test-utils/src/payload.rs#L37
    node.advance_block().await.unwrap();

    // We now are in fork 1, where only wallet 0 is allowed
    let transfer_tx_fork1_sender = transfer_bytes(0, None, wallet_0.clone()).await;
    node.rpc.inject_tx(transfer_tx_fork1_sender).await.unwrap();

    // This tx will be valid in fork 2, but not fork 1
    let transfer_tx_from_fork2_to =
        transfer_bytes(1, Some(wallet_1.address()), wallet_5.clone()).await;
    let transfer_error = node.rpc.inject_tx(transfer_tx_from_fork2_to.clone()).await.unwrap_err();
    assert_unsupported_tx(transfer_error);

    // Continue through fork 1
    node.advance_block().await.unwrap();
    let transfer_tx_fork1_sender = transfer_bytes(1, None, wallet_0.clone()).await;
    node.rpc.inject_tx(transfer_tx_fork1_sender).await.unwrap();

    // Start fork 2
    node.advance_block().await.unwrap();
    // This transfer now works
    node.rpc.inject_tx(transfer_tx_from_fork2_to).await.unwrap();
    // A transfer from the fork 1 sender does not work though
    let transfer_tx_fork1_sender =
        transfer_bytes(2, Some(wallet_5.address()), wallet_0.clone()).await;
    node.rpc.inject_tx(transfer_tx_fork1_sender).await.unwrap_err();
    node.advance_block().await.unwrap();

    // Continue through fork 2
    let transfer_tx_fork2_to = transfer_bytes(0, Some(wallet_1.address()), wallet_3.clone()).await;
    node.rpc.inject_tx(transfer_tx_fork2_to).await.unwrap();
    node.advance_block().await.unwrap();

    // Still works on the last block
    let transfer_tx_fork2_to = transfer_bytes(0, Some(wallet_1.address()), wallet_2.clone()).await;
    node.rpc.inject_tx(transfer_tx_fork2_to).await.unwrap();
    // A transfer to a random address does not work
    let transfer_tx = transfer_bytes(1, None, wallet_2.clone()).await;
    let transfer_error = node.rpc.inject_tx(transfer_tx).await.unwrap_err();
    assert_unsupported_tx(transfer_error);
    node.advance_block().await.unwrap();

    // Start fork 3, everything is allowed.
    // We show that everyone can send to a random address
    let transfer_tx = transfer_bytes(3, None, wallet_5.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    let transfer_tx = transfer_bytes(0, None, wallet_4.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    let transfer_tx = transfer_bytes(1, None, wallet_3.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    let transfer_tx = transfer_bytes(1, None, wallet_2.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    node.advance_block().await.unwrap();

    // And it keeps working into fork 3
    let transfer_tx = transfer_bytes(0, None, wallet_1.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    let transfer_tx = transfer_bytes(3, None, wallet_0.clone()).await;
    node.rpc.inject_tx(transfer_tx).await.unwrap();
    node.advance_block().await.unwrap();
}

#[tokio::test]
async fn pool_works() {
    ivm_test_utils::test_tracing();

    let tasks = TaskManager::current();
    let exec = tasks.executor();

    let genesis: Genesis =
        serde_json::from_str(include_str!("../../mock/eth-genesis.json")).unwrap();
    let chain_spec = Arc::new(
        ChainSpecBuilder::default()
            .chain(MAINNET.chain)
            .genesis(genesis.clone())
            .cancun_activated()
            .build(),
    );
    let node_config = NodeConfig::test()
        .with_chain(chain_spec)
        .with_unused_ports()
        .with_rpc(RpcServerArgs::default().with_unused_ports().with_http());

    let config = IvmConfig::allow_all();
    let ivm_node_types = IvmNode::new(config);
    let NodeHandle { node, node_exit_future: _ } = NodeBuilder::new(node_config.clone())
        .testing_node(exec.clone())
        .node(ivm_node_types)
        .launch()
        .await
        .unwrap();

    let mut node = NodeTestContext::new(node, eth_payload_attributes).await.unwrap();

    let wallets = Wallet::new(30).gen();
    let wallet_0 = wallets[29].clone();

    let normal_gas = 20e9 as u128;
    let tx0 = signed_bytes(1, 21000, 0, None, None, wallet_0.clone(), normal_gas, normal_gas).await;
    let tx1 = signed_bytes(1, 21000, 1, None, None, wallet_0.clone(), normal_gas, normal_gas).await;
    let tx1point1 =
        signed_bytes(1, 21001, 1, None, None, wallet_0.clone(), normal_gas, normal_gas).await;
    let tx2 = signed_bytes(1, 21000, 2, None, None, wallet_0.clone(), normal_gas, normal_gas).await;
    let tx3 = signed_bytes(1, 21000, 3, None, None, wallet_0.clone(), normal_gas, normal_gas).await;
    let tx4 = signed_bytes(1, 21000, 4, None, None, wallet_0.clone(), normal_gas, normal_gas).await;

    node.rpc.inject_tx(tx0).await.unwrap();
    node.rpc.inject_tx(tx1).await.unwrap();
    // Try injecting same nonce twice
    let err = node.rpc.inject_tx(tx1point1).await.unwrap_err().to_string();
    assert_eq!(err, *"replacement transaction underpriced");
    node.advance_block().await.unwrap();

    // See transaction pool state for all available methods
    assert_eq!(node.inner.pool().pool_size().pending, 0);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 0);
    assert_eq!(node.inner.pool().pool_size().total, 0);

    node.rpc.inject_tx(tx2).await.unwrap();
    // Skip nonce 3
    node.rpc.inject_tx(tx4).await.unwrap();
    node.advance_block().await.unwrap();

    assert_eq!(node.inner.pool().pool_size().pending, 0);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 1);
    assert_eq!(node.inner.pool().pool_size().total, 1);

    // Fill in the nonce gap
    node.rpc.inject_tx(tx3).await.unwrap();
    assert_eq!(node.inner.pool().pool_size().pending, 2);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 0);
    assert_eq!(node.inner.pool().pool_size().total, 2);
    node.advance_block().await.unwrap();
    // Pool is emptied because everything gets included
    assert_eq!(node.inner.pool().pool_size().pending, 0);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 0);
    assert_eq!(node.inner.pool().pool_size().total, 0);

    // There was a bug with validation logic that always returned u64::MAX
    // and any tip greater then that made the tx get stuck. Now we make sure
    // validated transaction always outputs U256::MAX to get around this issue.
    let over_u64_max = u128::MAX;
    let tx5 =
        signed_bytes(1, 21000, 5, None, None, wallet_0.clone(), over_u64_max, normal_gas).await;

    node.rpc.inject_tx(tx5).await.unwrap();
    assert_eq!(node.inner.pool().pool_size().pending, 1);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 0);
    assert_eq!(node.inner.pool().pool_size().total, 1);

    node.advance_block().await.unwrap();
    assert_eq!(node.inner.pool().pool_size().pending, 0);
    assert_eq!(node.inner.pool().pool_size().basefee, 0);
    assert_eq!(node.inner.pool().pool_size().queued, 0);
    assert_eq!(node.inner.pool().pool_size().total, 0);
}
