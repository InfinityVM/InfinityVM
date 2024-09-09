//! CLI for making HTTP request to clob node.

use clap::{Args, Parser};
use clap::Subcommand;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
  #[clap(subcommand)]
  commands: Commands
}

#[derive(Subcommand)]
enum Commands {
    /// Cancel an order.
    Cancel(CancelArgs),
    /// Fetch the CLOB state.
    ClobState,
    /// Place an order request.
    Order(OrderArgs),
    // / Withdraw funds.
    Withdraw(WithdrawArgs),
}

#[derive(Args)]
struct CancelArgs {
    /// ID of the order to cancel
    #[arg(short, long)]
    oid: u64,
}

#[derive(Args)]
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

#[derive(Args)]
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
