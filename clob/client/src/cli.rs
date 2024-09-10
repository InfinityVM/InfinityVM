//! CLI for making HTTP request to clob node.

use crate::Client;
use alloy::primitives::{hex::FromHex, Address};
use clap::{Args, Parser, Subcommand};
use clob_core::api::{AddOrderRequest, CancelOrderRequest, WithdrawRequest};
use clob_node::K256LocalSigner;
use alloy::hex;
use std::env;

const CLOB_PRIVATE_KEY: &str = "CLOB_PRIVATE_KEY";

/// CLI for interacting with the CLOB
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// HTTP endpoint.
    #[arg(short, long)]
    http_endpoint: String,
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    /// Run the CLI
    pub async fn run() -> eyre::Result<()> {
        let args = Self::parse();

        let client = Client::new(args.http_endpoint);

        match args.commands {
            Commands::Cancel(a) => {
                let result = client.cancel(CancelOrderRequest { oid: a.oid }).await?;
                println!("{result:?}");
            }
            Commands::ClobState => {
                let result = client.clob_state().await?;
                println!("{result:?}");
            }
            Commands::Order(a) => {
                let address = Address::from_hex(a.address).unwrap();
                let order = AddOrderRequest {
                    address: address.into(),
                    is_buy: a.is_buy,
                    limit_price: a.limit_price,
                    size: a.size,
                };
                let result = client.order(order).await?;
                println!("{result:?}");
            }
            Commands::Withdraw(a) => {
                let address = Address::from_hex(a.address).unwrap();
                let withdraw = WithdrawRequest {
                    address: address.into(),
                    base_free: a.base_free,
                    quote_free: a.quote_free,
                };
                let result = client.withdraw(withdraw).await?;
                println!("{result:?}");
            }
            Commands::Deposit(a) => {
                let private_key = private_key()?;
                let wallet = EthereumWallet::from(private_key);

                let provider = ProviderBuilder::new()
                    .with_recommended_fillers()
                    .wallet(wallet)
                    .on_http(a.http_endpoint.parse().unwrap());
            }
        };

        Ok(())
    }
}

fn private_key() -> eyre::Result<K256LocalSigner> {
    let private_key = env::var(CLOB_PRIVATE_KEY)?;
    let decoded = hex::decode(private_key)?;

    K256LocalSigner::from_slice(&decoded).map_err(Into::into)
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Cancel an order.
    Cancel(CancelArgs),
    /// Fetch the CLOB state.
    ClobState,
    /// Place an order request.
    Order(OrderArgs),
    /// Withdraw funds.
    Withdraw(WithdrawArgs),
    /// Deposit funds into the CLOB contract.
    Deposit(DepositArgs),
}

#[derive(Args, Debug)]
struct CancelArgs {
    /// ID of the order to cancel
    #[arg(short, long)]
    oid: u64,
}

#[derive(Args, Debug)]
struct OrderArgs {
    /// Address of the user placing the order.
    #[arg(short, long)]
    address: String,
    /// If this is a buy or sell order.
    #[arg(short, long)]
    is_buy: bool,
    /// Quote asset per unit of base asset.
    #[arg(short, long)]
    limit_price: u64,
    /// Size of the base asset to exchange.
    #[arg(short, long)]
    size: u64,
}

#[derive(Args, Debug)]
struct WithdrawArgs {
    /// Address of the user to withdraw from.
    #[arg(short, long)]
    address: String,
    /// Size of the base balance to withdraw.
    #[arg(short, long)]
    base_free: u64,
    /// Size of the quote asset to withdraw.
    #[arg(short, long)]
    quote_free: u64,
}

#[derive(Args, Debug)]
struct DepositArgs {
    /// Anvil account number to use for the key
    anvil_account: u32,
    /// Address of the clob contract.
    clob_contract: String,
    /// Quote asset balance.
    #[arg(short, long)]
    quote: u64,
    /// Base asset balance.
    #[arg(short, long)]
    base: u64, 
}
