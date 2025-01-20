//! IVM execution client.

use clap::Parser;
use ivm_exec::{
    config::IvmConfig,
    evm::IvmExecutorBuilder,
    payload::IvmPayloadBuilder,
    pool::{validator::IvmTransactionAllowConfig, IvmPoolBuilder},
    IvmAddOns, IvmCliExt,
};
use reth::cli::Cli;
use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_node_builder::{engine_tree_config::TreeConfig, EngineNodeLauncher};
use reth_node_ethereum::EthereumNode;

const IVM_CONFIG_FILE: &str = "ivm_config.toml";
const GIT_SHA: &'static str = env!("VERGEN_GIT_SHA", "vergen build time git sha missing.");
const GIT_BRANCH: &'static str = env!("VERGEN_GIT_BRANCH", "vergen build time git branch missing.");
const GIT_DESCRIBE: &'static str =
    env!("VERGEN_GIT_DESCRIBE", "vergen build time git describe missing.");

#[global_allocator]
static ALLOC: reth_cli_util::allocator::Allocator = reth_cli_util::allocator::new_allocator();

fn main() {
    if let Err(err) =
        Cli::<EthereumChainSpecParser, IvmCliExt>::parse().run(|builder, args| async move {
            tracing::info!(
                "ivm-exec git build info: sha={} describe={} branch={}",
                GIT_SHA,
                GIT_DESCRIBE,
                GIT_BRANCH
            );

            let transaction_allow = if args.allow_all {
                tracing::warn!("IVM Configuration overridden, all transactions will be allowed");
                IvmTransactionAllowConfig::with_all()
            } else {
                let ivm_config_path = if let Some(ivm_config) = args.ivm_config {
                    ivm_config
                } else {
                    builder.config().datadir().data_dir().join(IVM_CONFIG_FILE)
                };
                tracing::info!(path=?ivm_config_path, "IVM Configuration loading");
                let ivm_config = IvmConfig::from_path(&ivm_config_path)?;
                ivm_config.transaction_allow
            };
            let pool_builder = IvmPoolBuilder::new(transaction_allow);

            let handle = builder
                .with_types::<EthereumNode>()
                .with_components(
                    EthereumNode::components()
                        .pool(pool_builder)
                        .executor(IvmExecutorBuilder)
                        .payload(IvmPayloadBuilder::default()),
                )
                .with_add_ons(IvmAddOns::default())
                .launch_with_fn(|launch_builder| {
                    let launcher = EngineNodeLauncher::new(
                        launch_builder.task_executor().clone(),
                        launch_builder.config().datadir(),
                        TreeConfig::default(),
                    );
                    launch_builder.launch_with(launcher)
                })
                .await?;

            handle.wait_for_node_exit().await
        })
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
