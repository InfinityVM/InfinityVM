//! IVM execution client.

use clap::Parser;
use ivm_exec::{config::IvmConfig, pool::IvmPoolBuilder, IvmAddOns, IvmCliExt};
use reth::{chainspec::EthereumChainSpecParser, cli::Cli};
use reth_node_ethereum::EthereumNode;

const IVM_CONFIG_FILE: &str = "ivm_config.toml";

fn main() {
    Cli::<EthereumChainSpecParser, IvmCliExt>::parse()
        .run(|builder, args| async move {
            let ivm_config_path = if let Some(ivm_config) = args.ivm_config {
                ivm_config
            } else {
                builder.config().datadir().data_dir().join(IVM_CONFIG_FILE)
            };
            let ivm_config = IvmConfig::from_path(&ivm_config_path)?;
            tracing::info!(path=?ivm_config_path, "IVM Configuration loaded");
            let pool_builder = IvmPoolBuilder::new(ivm_config.transaction_allow);

            let handle = builder
                .with_types::<EthereumNode>()
                .with_components(EthereumNode::components().pool(pool_builder))
                .with_add_ons(IvmAddOns::default())
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
