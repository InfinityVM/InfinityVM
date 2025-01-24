//! IVM execution client.

use clap::Parser;
use ivm_exec::{config::IvmConfig, IvmCliExt, IvmNode};
use reth::cli::Cli;
use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_node_builder::{engine_tree_config::TreeConfig, EngineNodeLauncher};

const IVM_CONFIG_FILE: &str = "ivm_config.toml";

const GIT_SHA: &str = env!("VERGEN_GIT_SHA", "vergen build time git sha missing.");
const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH", "vergen build time git branch missing.");
const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE", "vergen build time git describe missing.");

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

            let ivm_config = if args.allow_all {
                tracing::warn!("IVM Configuration overridden, all transactions will be allowed");
                IvmConfig::allow_all()
            } else {
                let ivm_config_path = if let Some(ivm_config) = args.ivm_config {
                    ivm_config
                } else {
                    builder.config().datadir().data_dir().join(IVM_CONFIG_FILE)
                };
                tracing::info!(path=?ivm_config_path, "IVM Configuration loading");

                IvmConfig::from_path(&ivm_config_path)?
            };

            tracing::info!("Loaded with IVM config: {:?}", ivm_config);

            let ivm_node = IvmNode::new(ivm_config);

            let handle = builder
                .node(ivm_node)
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
