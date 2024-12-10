use ivm_exec::{pool::IvmPoolBuilder, IvmAddOns};
use reth::cli::Cli;
use reth_node_ethereum::EthereumNode;

fn main() {
    Cli::parse_args()
        .run(|builder, _| async move {
            let handle = builder
                .with_types::<EthereumNode>()
                // TODO: [now]: should we just use ethereum node and remove ivm node?
                .with_components(EthereumNode::components().pool(IvmPoolBuilder::default()))
                .with_add_ons(IvmAddOns::default())
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
