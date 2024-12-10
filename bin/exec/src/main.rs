use ivm_exec::{IvmAddOns, IvmNode};
use reth::cli::Cli;

fn main() {
    Cli::parse_args()
        .run(|builder, _| async move {
            let handle = builder
                .with_types::<IvmNode>()
                .with_components(IvmNode::components())
                .with_add_ons(IvmAddOns::default())
                .launch()
                .await?;

            handle.wait_for_node_exit().await
        })
        .unwrap();
}
