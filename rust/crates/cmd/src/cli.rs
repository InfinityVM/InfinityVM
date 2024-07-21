//! CLI for zkvm executor gRPC server.

use alloy::{primitives::{hex, Address}, signers::local::LocalSigner};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use clap::{Parser, Subcommand, ValueEnum};

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";

const DEFAULT_GRPC_MAX_RECV_MSG_SIZE: usize = 1024 * 1024 * 10;
const DEFAULT_GRPC_MAX_SEND_MSG_SIZE: usize = std::usize::MAX;

/// Errors from the gRPC Server CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not valid hex
    #[error("Environment variable {} must be set", ENV_RELAYER_PRIV_KEY)]
    RelayerPrivKeyNotSet,

}

#[derive(ValueEnum, Debug, Clone)]
enum LoggingFormat {
    Json,
    Text,
}


/// gRPC service.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {

    /// Logging level
    #[arg(
        long,
        default_value = "info"
    )]
    log_level: String,

    /// Logging format
    #[arg(
        long,
        default_value = "text"
    )]
    log_format: LoggingFormat,

    /// gRPC server address
    #[arg(
        long,
        default_value = "localhost:50051"
    )]
    grpc_address: String,

    /// gRPC gateway server address
    #[arg(
        long,
        default_value = "localhost:8080"
    )]
    grpc_gateway_address: String,

    // TODO: make required
    /// ZK shim address
    #[arg(long, required = true)]
    zk_shim_address: String,

    /// JobManager contract address
    #[arg(long, required = true)]
    job_manager_address: Address,

    /// Ethereum RPC address
    #[arg(long, required = true)]
    eth_rpc_address: String,
}

/// Command line interface for running the executor gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();    
    
        let relayer_private_key = std::env::var(ENV_RELAYER_PRIV_KEY)
        .map_err(|_| Error::RelayerPrivKeyNotSet)?;       

        Ok(())
    }
}
