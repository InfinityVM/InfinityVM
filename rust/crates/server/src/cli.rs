//! CLI for zkvm executor gRPC server.

use alloy::primitives::Address;
use std::net::SocketAddrV4;
use alloy::dyn_abi::DynSolType::Int;
use alloy::providers::{ProviderBuilder, WsConnect};
use alloy::rpc::types::Filter;
use alloy::rpc::types::trace::filter;
use alloy::sol;
use alloy::sol_types::SolValue;
use alloy::transports::{TransportError, TransportErrorKind};
use crate::service::Server;
use clap::{Parser, ValueEnum};
use tonic::codegen::tokio_stream::StreamExt;
use tonic::{IntoRequest, Status};
use proto::{ExecuteRequest, Job, JobInputs, SubmitJobRequest, SubmitJobResponse};
use proto::zkvm_executor_server::ZkvmExecutor;
use proto::zkvm_executor_client::ZkvmExecutorClient;
use proto::service_client::ServiceClient;

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

    #[error("event listener failure: {0}")]
    EventlistenerError(#[from] TransportError),

    #[error("event listener failure: {0}")]
    EventlistenerSolError(#[from] alloy::sol_types::Error),
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

    #[arg(long, required = true)]
    eth_rpc: String,
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

        // Parse the gRPC address
        let grpc_addr: SocketAddrV4 =
            opts.grpc_address.parse().map_err(|_| Error::InvalidGrpcAddress)?;

        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build gRPC reflection service");

        tokio::spawn(async move {
            event_subscriber(opts.eth_rpc,opts.job_manager_address).await
        });

        println!("Starting gRPC server at: {}", grpc_addr);
        tonic::transport::Server::builder()
            .add_service(proto::service_server::ServiceServer::new(Server::new()))
            .add_service(reflector)
            .serve(grpc_addr.into())
            .await
            .map_err(Into::into)

        // TODO: add HTTP gateway for gRPC server
    }
}


// TODO: fix
sol!(
    #[allow(missing_docs)]
    #[sol(rpc, bytecode = "")]
    contract JobManager {
        event JobCreated(uint32 indexed jobID, uint64 maxCycles, bytes indexed programID, bytes programInput);
    }
);

async fn event_subscriber(rpc_url: String, job_manager: Address) -> Result<(), Error>  {

    // Create the provider.
    let ws = WsConnect::new(rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await.map_err(|e|Error::EventlistenerError(e))?;

    let contract = JobManager::new(job_manager, provider.clone());

    let job_created_filter = contract.JobCreated_filter().watch().await.map_err(|e|Error::EventlistenerError(e))?;
    let mut stream = job_created_filter.into_stream().take(2);

    while let Some(other) = stream.next().await{
        let event = other.map_err(|e|Error::EventlistenerSolError(e))?;
        // TODO:

    }

    Ok(())
}