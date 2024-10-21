//! CLI for making HTTP request to clob node.

use crate::Client;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
};
use clap::{Args, Parser, Subcommand};
use simple_contracts::simple_consumer::SimpleConsumer;
use simple_core::api::SetValueRequest;
use contracts::get_default_deploy_info;
use eyre::OptionExt;
use test_utils::{get_account, get_signers};

/// CLI for interacting with the CLOB
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Simple node HTTP endpoint.
    #[arg(long, short = 'H', default_value = "http://127.0.0.1:40420")]
    simple_endpoint: String,
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    /// Run the CLI
    pub async fn run() -> eyre::Result<()> {
        let args = Self::parse();

        let client = Client::new(args.simple_endpoint.to_owned());

        match args.commands {
            Commands::SetValue(v) => {
                let local_signer = get_account(0);
                let address = local_signer.address();
                println!("account={}", address);

                let request = SetValueRequest {
                    value: v.value,
                };
                let result = client.set_value(request).await?;
                println!("{result:?}");

            }
            Commands::State => {
                let simple_state = client.state().await?;

                println!("value: {:?}", simple_state.latest_value());
            }
        };

        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Sets the Value
    SetValue(SetValueArgs),
    /// Queries the state
    State,
}

#[derive(Args, Debug)]
struct SetValueArgs {
    /// Value to set
    #[arg(short, long)]
    value: String,
}

#[derive(Args, Debug)]
struct QueryArgs {
    /// EVM node RPC address.
    #[arg(long, short, default_value = "http://127.0.0.1:60420")]
    eth_rpc: String,
}
