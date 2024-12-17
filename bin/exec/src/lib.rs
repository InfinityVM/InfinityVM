//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use reth::{
    builder::{rpc::RpcAddOns, FullNodeComponents, FullNodeTypes},
    network::NetworkHandle,
    rpc::eth::EthApi,
};
use std::path::PathBuf;

pub mod config;
pub mod engine;
pub mod pool;

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

/// Add IVM specific arguments to the default reth cli.
#[derive(Debug, Clone, clap::Args)]
pub struct IvmCliExt {
    /// Path to an IVM config toml file. Defaults to using a config in `<RETH
    /// DATADIR>/ivm_config.toml`. If no file is found one is generated.
    #[arg(long)]
    pub ivm_config: Option<PathBuf>,
}
