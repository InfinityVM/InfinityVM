//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use reth::{
    builder::{
        engine_tree_config::{DEFAULT_MEMORY_BLOCK_BUFFER_TARGET, DEFAULT_PERSISTENCE_THRESHOLD},
        rpc::RpcAddOns,
        FullNodeComponents, FullNodeTypes,
    },
    network::NetworkHandle,
    rpc::eth::EthApi,
};
use std::path::PathBuf;

pub mod config;
pub mod engine;
pub mod evm;
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

    /// Configure persistence threshold for the engine.
    // ref: https://github.com/paradigmxyz/reth/blob/c697543af058cb0571710b21c1b8980ba83967e9/bin/reth/src/main.rs#L37
    #[arg(long = "engine.persistence-threshold", default_value_t = DEFAULT_PERSISTENCE_THRESHOLD)]
    pub persistence_threshold: u64,

    /// Configure the target number of blocks to keep in memory.
    // ref: https://github.com/paradigmxyz/reth/blob/c697543af058cb0571710b21c1b8980ba83967e9/bin/reth/src/main.rs#L41
    #[arg(long = "engine.memory-block-buffer-target", default_value_t = DEFAULT_MEMORY_BLOCK_BUFFER_TARGET)]
    pub memory_block_buffer_target: u64,
}
