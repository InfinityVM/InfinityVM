use clap::{Arg, ArgMatches, Command};
use tracing_subscriber::{fmt, filter::EnvFilter, prelude::*};
use std::error::Error;
use std::str::FromStr;
use tonic::transport::Server;
use tracing::Level;
use tracing_subscriber::fmt::Subscriber as FmtSubscriber;
use alloy::primitives::Address;

// CLI flag and value constants
const LOG_LEVEL_JSON: &str = "json";
const LOG_LEVEL_TEXT: &str = "text";

const FLAG_LOG_LEVEL: &str = "log-level";
const FLAG_LOG_FORMAT: &str = "log-format";
const FLAG_GRPC_ADDRESS: &str = "grpc-address";
const FLAG_GRPC_GATEWAY_ADDRESS: &str = "grpc-gateway-address";
const FLAG_ZK_SHIM_ADDRESS: &str = "zk-shim-address";
const FLAG_JOB_MANAGER_ADDRESS: &str = "job-manager-address";
const FLAG_ETH_RPC_ADDRESS: &str = "eth-rpc-address";

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";

const DEFAULT_GRPC_MAX_RECV_MSG_SIZE: usize = 1024 * 1024 * 10;
const DEFAULT_GRPC_MAX_SEND_MSG_SIZE: usize = std::usize::MAX;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("infinity-server")
        .version("1.0")
        .about("infinity-server is a gRPC server that runs the InfinityVM async enshrined coprocessing service")
        .arg(
            Arg::new(FLAG_LOG_LEVEL)
                .long(FLAG_LOG_LEVEL)
                .help("logging level")
                .default_value("info"),
        )
        .arg(
            Arg::new(FLAG_LOG_FORMAT)
                .long(FLAG_LOG_FORMAT)
                .help("logging format [json|text]")
                .default_value(LOG_LEVEL_TEXT),
        )
        .arg(
            Arg::new(FLAG_GRPC_ADDRESS)
                .long(FLAG_GRPC_ADDRESS)
                .help("The gRPC server address")
                .default_value("localhost:50051"),
        )
        .arg(
            Arg::new(FLAG_GRPC_GATEWAY_ADDRESS)
                .long(FLAG_GRPC_GATEWAY_ADDRESS)
                .help("The gRPC gateway server address")
                .default_value("localhost:8080"),
        )
        .arg(
            Arg::new(FLAG_ZK_SHIM_ADDRESS)
                .long(FLAG_ZK_SHIM_ADDRESS)
                .help("The ZK shim address")
                .required(true),
        )
        .arg(
            Arg::new(FLAG_JOB_MANAGER_ADDRESS)
                .long(FLAG_JOB_MANAGER_ADDRESS)
                .help("The JobManager contract address")
                .required(true),
        )
        .arg(
            Arg::new(FLAG_ETH_RPC_ADDRESS)
                .long(FLAG_ETH_RPC_ADDRESS)
                .help("The Ethereum RPC address")
                .required(true),
        )
        .get_matches();

    let grpc_address: &String = matches.get_one(FLAG_GRPC_ADDRESS).unwrap();
    let grpc_gateway_address: &String = matches.get_one(FLAG_GRPC_GATEWAY_ADDRESS).unwrap();
    let zk_shim_address: &String = matches.get_one(FLAG_ZK_SHIM_ADDRESS).unwrap();

    let job_manager_address_string: &String = matches.get_one(FLAG_JOB_MANAGER_ADDRESS).unwrap();
    let job_manager_address = job_manager_address_string.parse::<Address>()
    .map_err(|_| format!("Invalid Ethereum address: {}", job_manager_address_string))?;

    let eth_rpc_address: &String = matches.get_one(FLAG_ETH_RPC_ADDRESS).unwrap();

    let relayer_private_key = std::env::var(ENV_RELAYER_PRIV_KEY)
    .map_err(|_| format!("Environment variable {} must be set", ENV_RELAYER_PRIV_KEY))?;

    println!("Starting gRPC server at: {}", grpc_address);
    println!("Starting gRPC gateway server at: {}", grpc_gateway_address);
    println!("ZK shim address: {}", zk_shim_address);
    println!("JobManager contract address: {}", job_manager_address);
    println!("Ethereum RPC address: {}", eth_rpc_address);
    println!("Relayer private key: {}", relayer_private_key);

    // You would set up your gRPC server and other components here.
    // For example:
    // let svc = MyGrpcService::default();
    // start_grpc_server(grpc_address, svc).await?;

    Ok(())
}
