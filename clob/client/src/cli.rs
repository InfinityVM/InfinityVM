//! CLI for making HTTP request to clob node.

use crate::Client;
use alloy::primitives::U256;
use alloy::{
    network::EthereumWallet,
    primitives::{hex::FromHex, Address},
    providers::ProviderBuilder,
};
use clap::{Args, Parser, Subcommand};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::{AddOrderRequest, CancelOrderRequest, WithdrawRequest};
use clob_test_utils::{clob_consumer_deploy, mint_and_approve};
use test_utils::get_signers;
use test_utils::job_manager_deploy;

/// CLI for interacting with the CLOB
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// HTTP endpoint.
    #[arg(long, short = 'H', default_value = "http://127.0.0.1:60420")]
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
                let all_wallets = get_signers((a.anvil_account + 1) as usize);
                let local_signer = all_wallets[a.anvil_account as usize].clone();
                println!("account={}", local_signer.address());

                let eth_wallet = EthereumWallet::from(local_signer);
                let provider = ProviderBuilder::new()
                    .with_recommended_fillers()
                    .wallet(eth_wallet)
                    .on_http(args.http_endpoint.parse().unwrap());

                let clob_contract = Address::from_hex(a.clob_contract).unwrap();
                let clob_consumer = ClobConsumer::new(clob_contract, &provider);
                let base_amount = U256::try_from(a.base).unwrap();
                let quote_amount = U256::try_from(a.quote).unwrap();

                let call = clob_consumer.deposit(base_amount, quote_amount);
                let tx_hash = call.send().await.unwrap().get_receipt().await.unwrap();
                println!("tx_hash={}", tx_hash.transaction_hash);
            }
            Commands::Deploy => {
                let job_manager_deploy = job_manager_deploy(args.http_endpoint.clone()).await;
                let clob_deploy = clob_consumer_deploy(
                    args.http_endpoint.clone(),
                    &job_manager_deploy.job_manager,
                )
                .await;
                mint_and_approve(&clob_deploy, args.http_endpoint.clone()).await;

                println!("job_manager: {}", job_manager_deploy.job_manager);
                println!("quote_erc20: {}", clob_deploy.quote_erc20);
                println!("base_erc20: {}", clob_deploy.base_erc20);
                println!("clob_consumer: {}", clob_deploy.clob_consumer);
            }
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
    #[arg(short = 'A', long)]
    anvil_account: u32,
    /// Address of the clob contract.
    #[arg(short, long, default_value = "0x78e6B135B2A7f63b281C80e2ff639Eed32E2a81b")]
    clob_contract: String,
    /// Address of quote token ERC20 contract.
    #[arg(short, long, default_value = "0x71a9d115E322467147391c4a71D85F8e1cA623EF")]
    quote_contract: String,
    /// Address of base token ERC20 contract.
    #[arg(short, long, default_value = "0x93C7a6D00849c44Ef3E92E95DCEFfccd447909Ae")]
    base_contract: String,
    /// Quote asset balance.
    #[arg(short = 'Q', long)]
    quote: u64,
    /// Base asset balance.
    #[arg(short = 'B', long)]
    base: u64,
}
