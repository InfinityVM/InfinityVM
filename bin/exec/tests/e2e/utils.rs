//! Utilities to help with e2e tests.

use alloy_network::eip2718::Encodable2718;
use alloy_primitives::{Address, Bytes, TxKind, B256, U256};
use alloy_rpc_types_engine::PayloadAttributes;
use alloy_rpc_types_eth::{TransactionInput, TransactionRequest};
use alloy_signer_local::PrivateKeySigner;
use reth::rpc::server_types::eth::{error::RpcPoolError, EthApiError, RpcInvalidTransactionError};
use reth_e2e_test_utils::transaction::TransactionTestContext;
use reth_ethereum_engine_primitives::EthPayloadBuilderAttributes;

pub(crate) fn eth_payload_attributes(timestamp: u64) -> EthPayloadBuilderAttributes {
    let attributes = PayloadAttributes {
        timestamp,
        prev_randao: B256::ZERO,
        suggested_fee_recipient: Address::ZERO,
        withdrawals: Some(vec![]),
        parent_beacon_block_root: Some(B256::ZERO),
    };
    EthPayloadBuilderAttributes::new(B256::ZERO, attributes)
}

pub(crate) fn assert_unsupported_tx(error: EthApiError) {
    match error {
        EthApiError::PoolError(RpcPoolError::Invalid(
            RpcInvalidTransactionError::TxTypeNotSupported,
        )) => (),
        _ => panic!(),
    };
}

/// Creates a type 2718 transaction
fn tx(
    chain_id: u64,
    gas: u64,
    nonce: u64,
    to: Option<Address>,
    data: Option<Bytes>,
) -> TransactionRequest {
    let to = to.unwrap_or_else(Address::random);
    TransactionRequest {
        nonce: Some(nonce),
        value: Some(U256::from(100)),
        to: Some(TxKind::Call(to)),
        gas: Some(gas),
        max_fee_per_gas: Some(20e9 as u128),
        max_priority_fee_per_gas: Some(20e9 as u128),
        chain_id: Some(chain_id),
        input: TransactionInput { input: None, data },
        authorization_list: None,

        ..Default::default()
    }
}

/// Creates a type 2718 transaction. Generates random address for to
/// if none is specified
pub(crate) async fn transfer_bytes(
    nonce: u64,
    to: Option<Address>,
    wallet: PrivateKeySigner,
) -> Bytes {
    let tx = tx(1, 21000, nonce, to, None);
    let signed = TransactionTestContext::sign_tx(wallet, tx).await;

    signed.encoded_2718().into()
}
