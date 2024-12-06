use reth::cli::Cli;
use reth_node_ethereum::{node::EthereumAddOns, EthereumNode};

fn main() {
    Cli::parse_args()
        .run(|builder, _| async move {
            let handle = builder
                // use the default ethereum node types
                .with_types::<EthereumNode>()
                // Configure the components of the node
                // use default ethereum components but use our custom pool
                // .with_components(EthereumNode::components().pool(CustomPoolBuilder::default()))
                .with_components(EthereumNode::components())
                .with_add_ons(EthereumAddOns::default())
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
