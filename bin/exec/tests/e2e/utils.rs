//! Utilities to help with e2e tests.

use alloy_primitives::{Address, B256};
use alloy_rpc_types_engine::PayloadAttributes;
use reth::rpc::server_types::eth::{error::RpcPoolError, EthApiError, RpcInvalidTransactionError};
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
