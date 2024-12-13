//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use reth::{
    builder::{rpc::RpcAddOns, FullNodeComponents, FullNodeTypes},
    network::NetworkHandle,
    rpc::eth::EthApi,
};

pub mod engine;

/// RPC add on that includes [`IvmEngineValidatorBuilder`].
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
