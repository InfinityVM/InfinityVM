//! CLI for making HTTP request to clob node.

use crate::Client;
use alloy::{
    network::EthereumWallet,
    primitives::{hex::FromHex, Address},
    providers::ProviderBuilder,
};
use clap::{Args, Parser, Subcommand};
use clob_core::api::{AddOrderRequest, CancelOrderRequest, WithdrawRequest};
use clob_node::K256LocalSigner;
use clob_test_utils::mock_erc20::MockErc20;
use test_utils::wallet::Wallet;

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

        let client = Client::new(args.http_endpoint.to_owned());

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
                let all_wallets = Wallet::new((a.anvil_account + 1) as usize).gen();
                let bytes = all_wallets[a.anvil_account as usize].to_bytes().0;
                let local_signer = K256LocalSigner::from_slice(&bytes).unwrap();
                println!("wallet={}", local_signer.address());

                let eth_wallet = EthereumWallet::from(local_signer);
                let provider = ProviderBuilder::new()
                    .with_recommended_fillers()
                    .wallet(eth_wallet)
                    .on_http(args.http_endpoint.parse().unwrap());

                if a.quote != 0 {
                    let address = Address::from_hex(a.quote_contract).unwrap();
                    let erc20 = MockErc20::new(address, &provider);
                }
            }
            Commands::Deploy => {}
        };

        Ok(())
    }
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
    Deploy,
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
    #[arg(short, long)]
    clob_contract: String,
    /// Address of quote token ERC20 contract.
    #[arg(short, long)]
    quote_contract: String,
    /// Address of base token ERC20 contract.
    #[arg(short, long)]
    base_contract: String,
    /// Quote asset balance.
    #[arg(short, long)]
    quote: u64,
    /// Base asset balance.
    #[arg(short, long)]
    base: u64,
}
