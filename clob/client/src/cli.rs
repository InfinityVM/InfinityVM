//! CLI for making HTTP request to clob node.

use crate::Client;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
};
use clap::{Args, Parser, Subcommand};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::{AddOrderRequest, CancelOrderRequest, WithdrawRequest};
use serde::{Deserialize, Serialize};
use test_utils::get_account;

/// Path to write deploy info to
pub const DEFAULT_DEPLOY_INFO: &str = "./logs/clob_deploy_info.json";

/// Contract deployment info for the clob.
#[derive(Serialize, Deserialize, Debug)]
pub struct ClobDeployInfo {
    /// Job Manager contract address.
    pub job_manager: Address,
    /// Quote ERC20 contract address.
    pub quote_erc20: Address,
    /// Base ERC20 contract address.
    pub base_erc20: Address,
    /// CLOB Consumer contract address.
    pub clob_consumer: Address,
}

/// CLI for interacting with the CLOB
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// CLOB HTTP endpoint.
    #[arg(long, short = 'H', default_value = "http://127.0.0.1:40420")]
    clob_endpoint: String,
    #[clap(subcommand)]
    commands: Commands,
}

impl Cli {
    /// Run the CLI
    pub async fn run() -> eyre::Result<()> {
        let args = Self::parse();

        let client = Client::new(args.clob_endpoint.to_owned());

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
                let local_signer = get_account(a.anvil_account as usize);
                let address = local_signer.address();
                println!("account={}", address);

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
                let address = get_account(a.anvil_account as usize).address();
                println!("account={}", address);

                let withdraw = WithdrawRequest {
                    address: address.into(),
                    base_free: a.base_free,
                    quote_free: a.quote_free,
                };
                let result = client.withdraw(withdraw).await?;
                println!("{result:?}");
            }
            Commands::Deposit(a) => {
                let deploy_info = get_default_deploy_info();

                let local_signer = get_account(a.anvil_account as usize);
                println!("account={}", local_signer.address());

                let eth_wallet = EthereumWallet::from(local_signer.clone());
                let provider = ProviderBuilder::new()
                    .with_recommended_fillers()
                    .wallet(eth_wallet)
                    .on_http(a.eth_rpc.parse().unwrap());

                let clob_consumer = ClobConsumer::new(deploy_info.clob_consumer, &provider);
                let base_amount = U256::try_from(a.base).unwrap();
                let quote_amount = U256::try_from(a.quote).unwrap();

                let call = clob_consumer.deposit(base_amount, quote_amount);
                let receipt = call.send().await.unwrap().get_receipt().await.unwrap();

                print_onchain_balances(
                    local_signer.address(),
                    a.eth_rpc,
                    deploy_info.clob_consumer,
                )
                .await;

                for log in receipt.inner.logs() {
                    let l = match log.log_decode::<ClobConsumer::Deposit>() {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let event = l.data();
                    println!(
                        "deposit.user={:?}, deposit.base={}, deposit.quote={}",
                        event.user, event.baseAmount, event.quoteAmount
                    );
                }

                println!("receipt.status={:?}", receipt.status());
            }
            Commands::Query(a) => {
                let address = get_account(a.anvil_account as usize).address();
                let deploy_info = get_default_deploy_info();

                println!("account={}", address);

                print_onchain_balances(address, a.eth_rpc, deploy_info.clob_consumer).await;

                let bytes = *address;
                let clob_state = client.clob_state().await?;
                let clob_base = clob_state.base_balances().get(&*bytes).unwrap();
                let clob_quote = clob_state.quote_balances().get(&*bytes).unwrap();

                println!("clob base: {:?}", clob_base);
                println!("clob quote: {:?}", clob_quote);
            }
            Commands::Deploy(_a) => {
                unimplemented!()
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
    /// Deploy job manager and clob consumer contracts. Also mint erc20 tokens to users
    Deploy(DeployArgs),
    /// Query balances onchain and in the clob.
    Query(QueryArgs),
}

#[derive(Args, Debug)]
struct CancelArgs {
    /// ID of the order to cancel
    #[arg(short, long)]
    oid: u64,
}

#[derive(Args, Debug)]
struct OrderArgs {
    /// Anvil account number to use for the key
    #[arg(short = 'A', long)]
    anvil_account: u32,
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
    #[arg(short = 'A', long)]
    anvil_account: u32,
    /// Size of the base balance to withdraw.
    #[arg(short = 'B', long)]
    base_free: u64,
    /// Size of the quote asset to withdraw.
    #[arg(short = 'Q', long)]
    quote_free: u64,
}

#[derive(Args, Debug)]
struct DepositArgs {
    /// Anvil account number to use for the key
    #[arg(short = 'A', long)]
    anvil_account: u32,
    /// Quote asset balance.
    #[arg(short = 'Q', long)]
    quote: u64,
    /// Base asset balance.
    #[arg(short = 'B', long)]
    base: u64,
    /// EVM node RPC address.
    #[arg(long, short, default_value = "http://127.0.0.1:60420")]
    eth_rpc: String,
}

#[derive(Args, Debug)]
struct DeployArgs {
    /// EVM node RPC address.
    #[arg(long, short, default_value = "http://127.0.0.1:60420")]
    eth_rpc: String,
    #[arg(long, short, default_value = "http://127.0.0.1:50420")]
    coproc_grpc: String,
}

#[derive(Args, Debug)]
struct QueryArgs {
    /// Anvil account number to use for the key
    #[arg(short = 'A', long)]
    anvil_account: u32,
    /// EVM node RPC address.
    #[arg(long, short, default_value = "http://127.0.0.1:60420")]
    eth_rpc: String,
}

async fn print_onchain_balances(account: Address, eth_rpc: String, clob_consumer: Address) {
    let provider =
        ProviderBuilder::new().with_recommended_fillers().on_http(eth_rpc.parse().unwrap());

    let clob_consumer = ClobConsumer::new(clob_consumer, &provider);

    let deposited_base = clob_consumer.depositedBalanceBase(account).call().await.unwrap()._0;
    println!("onchain: deposited base {}", deposited_base);

    let deposited_quote = clob_consumer.depositedBalanceQuote(account).call().await.unwrap()._0;
    println!("onchain: deposited quote {}", deposited_quote);

    let free_base = clob_consumer.freeBalanceBase(account).call().await.unwrap()._0;
    println!("onchain: free base {}", free_base);

    let locked_base = clob_consumer.lockedBalanceBase(account).call().await.unwrap()._0;
    println!("onchain: locked_base {}", locked_base);

    let free_quote = clob_consumer.freeBalanceQuote(account).call().await.unwrap()._0;
    println!("onchain: free quote {}", free_quote);

    let locked_quote = clob_consumer.lockedBalanceQuote(account).call().await.unwrap()._0;
    println!("onchain: locked quote {}", locked_quote);
}

fn get_default_deploy_info() -> ClobDeployInfo {
    let filename = DEFAULT_DEPLOY_INFO.to_string();
    let raw_json = std::fs::read(filename).unwrap();
    serde_json::from_slice(&raw_json).unwrap()
}
