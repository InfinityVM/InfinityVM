//! IVM execution client.

use ivm_exec::{IvmAddOns, pool::IvmPoolBuilder, IvmCliExt, config::IvmConfig};
use reth::cli::Cli;
use reth_node_ethereum::EthereumNode;
use reth::chainspec::EthereumChainSpecParser;
use clap::Parser;

const IVM_CONFIG_FILE: &str = "ivm_config.toml";

fn main() {
    Cli::<EthereumChainSpecParser, IvmCliExt>::parse()
        .run(|builder, args| async move {

            let ivm_config_path = if let Some(ivm_config) =  args.ivm_config {
                ivm_config
            } else {
                builder.config().datadir().data_dir().join(IVM_CONFIG_FILE)
            };
            IvmConfig::from_path(&ivm_config_path)?;
            tracing::info!(path=?ivm_config_path, "IVM Configuration loaded",);

            let handle = builder
                .with_types::<EthereumNode>()
                .with_components(EthereumNode::components().pool(IvmPoolBuilder))
                .with_add_ons(IvmAddOns::default())
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
