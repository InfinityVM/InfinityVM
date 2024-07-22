//! CLI for zkvm executor gRPC server.

use alloy::primitives::Address;
use crate::http::run_http_server;
use crate::service::Server;
use clap::{Parser, ValueEnum};
use std::net::{SocketAddr, SocketAddrV4};

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";

/// Errors from the gRPC Server CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not valid hex
    #[error("Environment variable {} must be set", ENV_RELAYER_PRIV_KEY)]
    RelayerPrivKeyNotSet,
    /// invalid gRPC address
    #[error("Invalid gRPC address")]
    InvalidGrpcAddress,
    /// grpc server failure
    #[error("grpc server failure: {0}")]
    GrpcServer(#[from] tonic::transport::Error),
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
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Logging format
    #[arg(long, default_value = "text")]
    log_format: LoggingFormat,

    /// gRPC server address
    #[arg(long, default_value = "127.0.0.1:50051")]
    grpc_address: String,

    /// gRPC gateway server address
    #[arg(long, default_value = "127.0.0.1:8080")]
    grpc_gateway_address: String,

    /// ZK shim address
    #[arg(long, required = true)]
    zk_shim_address: String,

    /// `JobManager` contract address
    #[arg(long, required = true)]
    job_manager_address: Address,

    /// Ethereum RPC address
    #[arg(long, required = true)]
    eth_rpc_address: String,
}

/// Command line interface for running the gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        // TODO (Maanav): add logging

        let _relayer_private_key =
            std::env::var(ENV_RELAYER_PRIV_KEY).map_err(|_| Error::RelayerPrivKeyNotSet)?;

        // Parse the addresses for gRPC server and gateway
        let grpc_addr: SocketAddrV4 =
            opts.grpc_address.parse().map_err(|_| Error::InvalidGrpcAddress)?;
        let grpc_addr_str = format!("http://{}", grpc_addr);
        let grpc_gateway_addr: SocketAddr = opts.grpc_gateway_address.parse().unwrap();

        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build gRPC reflection service");

        println!("Starting gRPC server at: {}", grpc_addr);
        let grpc_server = tonic::transport::Server::builder()
            .add_service(proto::service_server::ServiceServer::new(Server::new()))
            .add_service(reflector)
            .serve(grpc_addr.into());

        // Start HTTP gateway for gRPC server
        println!("Starting HTTP server at: {}", grpc_gateway_addr);
        let http_server = run_http_server(grpc_addr_str, grpc_gateway_addr);

        tokio::select! {
            res = grpc_server => {
                if let Err(e) = res {
                    eprintln!("gRPC server error: {}", e);
                }
            }
            res = http_server => {
                if let Err(e) = res {
                    eprintln!("HTTP server error: {}", e);
                }
            }
        }

        Ok(())
    }
}
