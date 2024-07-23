//! HTTP gateway is a reverse proxy that exposes an HTTP interface to the coprocessor-node gRPC routes.

use clap::Parser;
use std::net::SocketAddr;

pub mod gateway;

/// Error for http gateway
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Failure to connect to gRPC server
    #[error("failed to connect to grpc server: {0}")]
    ConnectionFailure(String),
    /// Network IO error
    #[error(transparent)]
    StdIO(#[from] std::io::Error),
    /// invalid gRPC address
    #[error("invalid gRPC address: {0}")]
    InvalidGrpcAddress(#[from] std::net::AddrParseError),
}

/// CLI options for running the gate
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// gRPC server address to proxy request to
    #[arg(long, default_value = "127.0.0.1:50051")]
    grpc_address: String,

    /// Address to listen on for HTTP requests
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen_address: String,
}

/// Command line interface for running the HTTP gateway.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the HTTP gateway.
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        let grpc_address: SocketAddr = opts.grpc_address.parse()?;
        let grpc_url = format!("http://{}", grpc_address);

        let listen_address: SocketAddr = opts.listen_address.parse()?;

        gateway::HttpGrpcGateway::new(grpc_url, listen_address).serve().await
    }
}
