//! IVM execution client.

use clap::Parser;
use ivm_exec::{
    config::IvmConfig, evm::IvmExecutorBuilder, pool::IvmPoolBuilder, IvmAddOns, IvmCliExt,
};
use reth::{
    builder::{engine_tree_config::TreeConfig, EngineNodeLauncher},
    chainspec::EthereumChainSpecParser,
    cli::Cli,
};
use reth_node_ethereum::EthereumNode;

const IVM_CONFIG_FILE: &str = "ivm_config.toml";

fn main() {
    if let Err(err) =
        Cli::<EthereumChainSpecParser, IvmCliExt>::parse().run(|builder, args| async move {
            let ivm_config_path = if let Some(ivm_config) = args.ivm_config {
                ivm_config
            } else {
                builder.config().datadir().data_dir().join(IVM_CONFIG_FILE)
            };
            tracing::info!(path=?ivm_config_path, "IVM Configuration loading");
            let ivm_config = IvmConfig::from_path(&ivm_config_path)?;
            let pool_builder = IvmPoolBuilder::new(ivm_config.transaction_allow);

            let handle = builder
                .with_types::<EthereumNode>()
                .with_components(
                    EthereumNode::components().pool(pool_builder).executor(IvmExecutorBuilder),
                )
                .with_add_ons(IvmAddOns::default())
                // Use engine v2.
                // launch code ref: https://github.com/paradigmxyz/reth/blob/c697543af058cb0571710b21c1b8980ba83967e9/bin/reth/src/main.rs#L79
                // engine v2 tracking issue ref: https://github.com/paradigmxyz/reth/issues/8742
                .launch_with_fn(|builder| {
                    let engine_tree_config = TreeConfig::default()
                        .with_persistence_threshold(args.persistence_threshold)
                        .with_memory_block_buffer_target(args.memory_block_buffer_target);
                    let launcher = EngineNodeLauncher::new(
                        builder.task_executor().clone(),
                        builder.config().datadir(),
                        engine_tree_config,
                    );
                    builder.launch_with(launcher)
                })
                .await
                .unwrap();

            handle.wait_for_node_exit().await
        })
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
