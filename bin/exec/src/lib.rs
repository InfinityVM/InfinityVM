//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use reth_network::NetworkHandle;
use reth_node_builder::{rpc::RpcAddOns, FullNodeComponents, FullNodeTypes};
use reth_rpc::eth::EthApi;
use std::path::PathBuf;

pub mod config;
pub mod engine;
pub mod evm;
pub mod payload;
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

    /// Do not enforce the transaction allow config; allow transactions from all senders.
    /// This will override values in the IVM config.
    #[arg(long = "tx-allow.all")]
    pub allow_all: bool,
}
