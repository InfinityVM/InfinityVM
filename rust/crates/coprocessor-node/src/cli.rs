//! CLI for coprocessor-node.

use crate::{
    job_processor::JobProcessorService,
    metrics::Metrics,
    relayer::{self, JobRelayerBuilder},
    service::CoprocessorNodeServerInner,
};
use alloy::{
    primitives::{hex, Address},
    signers::local::LocalSigner,
};
use async_channel::{bounded, Receiver, Sender};
use axum::{routing::get, Router, Server};
use clap::{Parser, Subcommand, ValueEnum};
use k256::ecdsa::SigningKey;
use prometheus::{Encoder, Registry, TextEncoder};
use proto::{coprocessor_node_server::CoprocessorNodeServer, Job};
use std::{
    net::{SocketAddr, SocketAddrV4},
    path::PathBuf,
    sync::Arc,
};
use tracing::info;
use zkvm_executor::{service::ZkvmExecutorService, DEV_SECRET};

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";
const ENV_ZKVM_OPERATOR_PRIV_KEY: &str = "ZKVM_OPERATOR_PRIV_KEY";

/// Errors from the gRPC Server CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not set
    #[error("environment variable {} must be set", ENV_RELAYER_PRIV_KEY)]
    RelayerPrivKeyNotSet,
    /// private key was not set
    #[error(
        "environment variable {} must be set, or specify operator subcommand",
        ENV_ZKVM_OPERATOR_PRIV_KEY
    )]
    OperatorPrivKeyNotSet,
    /// invalid gRPC address
    #[error("invalid gRPC address")]
    InvalidGrpcAddress,
    /// invalid prometheus address
    #[error("invalid prometheus address")]
    InvalidPromAddress,
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
    SignerLocal(#[from] alloy::signers::local::LocalSignerError),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
    /// relayer error
    #[error("relayer error: {0}")]
    Relayer(#[from] relayer::Error),
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

    /// prometheus metrics address
    #[arg(long, default_value = "127.0.0.1:3001")]
    prom_address: String,

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
    operator_key: Option<Operator>,

    /// Number of worker threads to use for processing jobs
    #[arg(long, default_value = "4")]
    worker_count: usize,
}

impl Opts {
    fn operator_signer(&self) -> Result<K256LocalSigner, Error> {
        let signer = match &self.operator_key {
            Some(Operator::Dev) => {
                info!("zkvm operator using development key");
                K256LocalSigner::from_slice(&DEV_SECRET)?
            }
            Some(Operator::KeyStore(KeyStore { path, password })) => {
                K256LocalSigner::decrypt_keystore(path, password)?
            }
            Some(Operator::Secret(Secret { secret })) => Self::signer_from_hex(secret)?,
            None => {
                let secret = std::env::var(ENV_ZKVM_OPERATOR_PRIV_KEY)
                    .map_err(|_| Error::OperatorPrivKeyNotSet)?;
                Self::signer_from_hex(&secret)?
            }
        };

        Ok(signer)
    }

    fn relayer_signer(&self) -> Result<K256LocalSigner, Error> {
        let secret =
            std::env::var(ENV_RELAYER_PRIV_KEY).map_err(|_| Error::RelayerPrivKeyNotSet)?;
        Self::signer_from_hex(&secret)
    }

    fn signer_from_hex(secret: &String) -> Result<K256LocalSigner, Error> {
        if secret.as_bytes().len() < 64 {
            return Err(Error::ShortPrivateKeyHex);
        }

        let decoded = hex::decode(secret)?;
        K256LocalSigner::from_slice(&decoded).map_err(Into::into)
    }
}

/// Command line interface for running the gRPC server.
#[derive(Parser, Debug)]
pub struct Cli;

impl Cli {
    /// Run the CLI
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        // Setup Prometheus registry & custom metrics
        let registry = Registry::new();
        let metrics = Arc::new(Metrics::new(&registry));

        // Create the metrics router
        let metrics_router = Router::new().route(
            "/metrics",
            get(move || {
                let registry = registry.clone();
                // TODO: Unclear if there's a better way to do this with a builder?
                // See reference https://github.com/tikv/rust-prometheus/blob/master/examples/example_custom_registry.rs#L17-L42
                // and https://github.com/tokio-rs/axum/blob/main/examples/prometheus-metrics/src/main.rs
                async move {
                    // let metric_familes = registry.gather();
                    let mut buffer = Vec::new();
                    TextEncoder::new().encode(&registry.gather(), &mut buffer).unwrap();
                    String::from_utf8(buffer).unwrap()
                }
            }),
        );

        // Start Prometheus server
        let prom_addr: SocketAddr =
            opts.prom_address.parse().map_err(|_| Error::InvalidPromAddress)?;
        tokio::spawn(async move {
            info!("Prometheus server listening on {}", prom_addr);
            // TODO: Should we be handling this error?
            Server::bind(&prom_addr).serve(metrics_router.into_make_service()).await.unwrap();
        });

        // Parse the addresses for gRPC server and gateway
        let grpc_addr: SocketAddrV4 =
            opts.grpc_address.parse().map_err(|_| Error::InvalidGrpcAddress)?;

        let zkvm_operator = opts.operator_signer()?;
        let relayer = opts.relayer_signer()?;

        info!("👷🏻 zkvm operator is {:?}", zkvm_operator.address());
        info!("✉️ relayer is {:?}", relayer.address());

        let db = db::init_db(opts.db_dir.clone())?;
        info!(db_path = opts.db_dir, "💾 db initialized");

        let executor = ZkvmExecutorService::new(zkvm_operator, opts.chain_id);

        // Initialize the async channels
        let (exec_queue_sender, exec_queue_receiver): (Sender<Job>, Receiver<Job>) = bounded(100);

        let job_relayer = JobRelayerBuilder::new()
            .signer(relayer)
            .build(opts.eth_rpc_address.clone(), opts.job_manager_address)?;
        let job_relayer = Arc::new(job_relayer);

        // Start the job processor with a specified number of worker threads.
        // The job processor stores a JoinSet which has a handle to each task.
        let mut job_processor = JobProcessorService::new(
            db,
            exec_queue_sender,
            exec_queue_receiver,
            job_relayer,
            executor,
            metrics.clone(),
        );
        job_processor.start(opts.worker_count).await;

        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
            .build()
            .expect("failed to build gRPC reflection service");

        let coprocessor_node_server =
            CoprocessorNodeServer::new(CoprocessorNodeServerInner { job_processor });

        tracing::info!("🚥 starting gRPC server at {}", grpc_addr);
        tonic::transport::Server::builder()
            .add_service(coprocessor_node_server)
            .add_service(reflector)
            .serve(grpc_addr.into())
            .await?;

        Ok(())
    }
}
