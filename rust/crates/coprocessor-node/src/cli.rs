//! CLI for coprocessor-node.

use crate::{job_processor::JobProcessorService, service::CoprocessorNodeServerInner};
use alloy::{
    primitives::{hex, Address},
    signers::local::LocalSigner,
};
use async_channel::{bounded, Receiver, Sender};
use clap::{Parser, Subcommand, ValueEnum};
use k256::ecdsa::SigningKey;
use proto::{coprocessor_node_server::CoprocessorNodeServer, Job};
use std::{net::SocketAddr, net::SocketAddrV4, path::PathBuf};
use tracing::info;
use zkvm_executor::{service::ZkvmExecutorService, DEV_SECRET};

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
    /// private key was not valid hex
    #[error("private key was not valid hex")]
    InvalidPrivateKeyHex(#[from] hex::FromHexError),
    /// private key hex was too short
    #[error("private key hex was too short")]
    ShortPrivateKeyHex,
    /// invalid private key
    #[error("invalid private key: {0}")]
    Ecdsa(#[from] k256::ecdsa::Error),
    /// error creating alloy signer
    #[error("error creating signer: {0}")]
    Signer(#[from] alloy::signers::Error),
    /// errors from alloy signer local crate
    #[error(transparent)]
    SignerLocal(#[from] alloy_signer_local::LocalSignerError),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
}

#[derive(ValueEnum, Debug, Clone)]
enum LoggingFormat {
    Json,
    Text,
}

type K256LocalSigner = LocalSigner<SigningKey>;

#[derive(Debug, Clone, Subcommand)]
#[command(version, about, long_about = None)]
enum Operator {
    /// Use a development key
    Dev,
    /// Use an encrypted keystore
    KeyStore(KeyStore),
    /// Pass the hex encoded secret in at the command line
    Secret(Secret),
}

#[derive(Debug, Clone, clap::Args)]
struct KeyStore {
    /// Path to JSON keystore
    #[arg(long, value_name = "FILE")]
    path: PathBuf,
    /// Password for decrypting the JSON keystore
    #[arg(long)]
    password: String,
}

#[derive(Debug, Clone, clap::Args)]
struct Secret {
    secret: String,
}

fn db_dir() -> String {
    let mut p = home::home_dir().expect("could not find users home dir");
    p.push(".config");
    p.push("ethos");
    p.push("networks");
    p.push("ethos-dev0");
    p.push("coprocessor-node");
    p.push("db");
    p.into_os_string().into_string().expect("could not create default db path")
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

    /// `JobManager` contract address
    #[arg(long, required = true)]
    job_manager_address: Address,

    /// Ethereum RPC address
    #[arg(long, required = true)]
    eth_rpc_address: String,

    /// Chain ID of where results are expected to get submitted.
    #[arg(long, required = true)]
    chain_id: Option<u64>,

    /// Path to the directory to include db
    #[arg(
        long,
        default_value_t = db_dir()
    )]
    db_dir: String,

    /// Operator key to use for signing
    #[command(subcommand)]
    operator_key: Operator,

    /// Number of worker threads to use for processing jobs
    #[arg(long, default_value = "4")]
    worker_count: usize,

    /// Address to listen on for HTTP requests
    #[arg(long, default_value = "127.0.0.1:8080")]
    grpc_gateway_address: String,
}

impl Opts {
    fn operator_signer(&self) -> Result<K256LocalSigner, Error> {
        let signer = match &self.operator_key {
            Operator::Dev => K256LocalSigner::from_slice(&DEV_SECRET)?,
            Operator::KeyStore(KeyStore { path, password }) => {
                K256LocalSigner::decrypt_keystore(path, password)?
            }
            Operator::Secret(Secret { secret }) => {
                if secret.as_bytes().len() < 64 {
                    return Err(Error::ShortPrivateKeyHex);
                }
                let hex = if secret[0..2] == *"0x" { &secret[2..] } else { &secret[..] };
                let decoded = hex::decode(hex)?;
                K256LocalSigner::from_slice(&decoded)?
            }
        };

        Ok(signer)
    }
}

/// Command line interface for running the gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        let _relayer_private_key =
            std::env::var(ENV_RELAYER_PRIV_KEY).map_err(|_| Error::RelayerPrivKeyNotSet)?;

        let grpc_address: SocketAddr = opts.grpc_address.parse()?;

        // Parse the addresses for gRPC server and gateway
        let grpc_addr: SocketAddrV4 = grpc_address.map_err(|_| Error::InvalidGrpcAddress)?;

        let signer = opts.operator_signer()?;

        let db = db::init_db(opts.db_dir.clone())?;
        info!(db_path = opts.db_dir, "db initialized");

        let executor = ZkvmExecutorService::new(signer, opts.chain_id);
        info!(
            chain_id = opts.chain_id,
            zkvm_operator_address = executor.signer_address().to_string(),
            "executor initialized"
        );

        // Initialize the async channels
        let (exec_queue_sender, exec_queue_receiver): (Sender<Job>, Receiver<Job>) = bounded(100);
        // TODO: broadcast_queue_receiver is not used right now, but should be passed into relayer
        // once that is added. This will remove the `Failed to push job` error log when running
        // tests.
        let (broadcast_queue_sender, _): (Sender<Job>, Receiver<Job>) = bounded(100);

        // Start the job processor with a specified number of worker threads.
        // The job processor stores a JoinSet which has a handle to each task.
        let mut job_processor = JobProcessorService::new(
            db,
            exec_queue_sender,
            exec_queue_receiver,
            broadcast_queue_sender,
            executor,
        );
        job_processor.start(opts.worker_count).await;

        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build gRPC reflection service");

        let coprocessor_node_server =
            CoprocessorNodeServer::new(CoprocessorNodeServerInner { job_processor });

        tracing::info!("starting gRPC server at {}", grpc_addr);
        tonic::transport::Server::builder()
            .add_service(coprocessor_node_server)
            .add_service(reflector)
            .serve(grpc_addr.into())
            .await?;

        let grpc_url = format!("http://{}", grpc_address);
        let grpc_gateway_address: SocketAddr = opts.grpc_gateway_address.parse()?;

        gateway::HttpGrpcGateway::new(grpc_url, grpc_gateway_address).serve().await?;

        Ok(())
    }
}
