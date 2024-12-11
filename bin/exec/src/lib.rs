//! IVM execution client types for plugging into reth node builder.

use reth::{
    builder::{rpc::RpcAddOns, FullNodeComponents, FullNodeTypes},
    network::NetworkHandle,
    rpc::eth::EthApi,
};
use crate::engine::IvmEngineValidatorBuilder;

pub mod engine;
pub mod pool;

/// RPC add ons that include [`IvmEngineValidatorBuilder`].
pub type IvmAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    IvmEngineValidatorBuilder,
>;
