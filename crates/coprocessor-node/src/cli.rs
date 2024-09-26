//! CLI for coprocessor-node.

use crate::{
    job_processor::JobProcessorConfig,
    node::{self, NodeConfig, WsConfig},
};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{hex, Address},
    signers::local::LocalSigner,
};
use clap::{Parser, Subcommand};
use k256::ecdsa::SigningKey;
use std::path::PathBuf;
use tracing::{info, instrument};
use zkvm_executor::DEV_SECRET;

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
    /// invalid http address
    #[error("invalid http address")]
    InvalidHttpAddress,
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
    /// error running node
    #[error(transparent)]
    Node(#[from] crate::node::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] db::Error),
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
    p.push("ivm");
    p.push("networks");
    p.push("ivm-dev0");
    p.push("coprocessor-node");
    p.push("db");
    p.into_os_string().into_string().expect("could not create default db path")
}

/// gRPC service.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// gRPC server address
    #[arg(long, default_value = "127.0.0.1:50051")]
    grpc_address: String,

    /// Address to listen on for the REST gRPC gateway
    #[arg(long, default_value = "127.0.0.1:8080")]
    http_address: String,

    /// prometheus metrics address
    #[arg(long, default_value = "127.0.0.1:3001")]
    prom_address: String,

    /// `JobManager` contract address
    #[arg(long, required = true)]
    job_manager_address: Address,

    /// HTTP Ethereum RPC address. Defaults to a local anvil node address.
    #[arg(long, default_value = "http://127.0.0.1:8545")]
    http_eth_rpc: String,

    /// WS Ethereum RPC address. Defaults to a local anvil node address.
    #[arg(long, default_value = "ws://127.0.0.1:8545")]
    ws_eth_rpc: String,

    /// WS Ethereum RPC retry backoff duration limit in milliseconds.
    #[arg(long, default_value_t = 5 * 60 * 1_000)]
    ws_backoff_limit_ms: u64,

    /// WS Ethereum RPC retry backoff multiplier. The sleep duration will be `num_retrys *
    /// backoff_multiplier_ms`.
    #[arg(long, default_value_t = 10)]
    ws_backoff_multiplier_ms: u64,

    /// Chain ID of where results are expected to get submitted. Defaults to anvil node chain id.
    #[arg(long, default_value = "31337")]
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
    #[arg(long, default_value_t = 32)]
    worker_count: usize,

    /// Max number of retries for relaying a job
    #[arg(long, default_value_t = 3)]
    max_retries: usize,

    /// Max size for the exec queue
    #[arg(long, default_value_t = 256)]
    exec_queue_bound: usize,

    /// Block to start syncing from.
    // TODO: https://github.com/Ethos-Works/InfinityVM/issues/142
    #[arg(long, default_value_t = BlockNumberOrTag::Earliest)]
    job_sync_start: BlockNumberOrTag,

    /// Required confirmations for tx
    #[arg(long, default_value_t = 1)]
    confirmations: u64,
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
    #[instrument]
    pub async fn run() -> Result<(), Error> {
        let opts = Opts::parse();

        let db = db::init_db(&opts.db_dir)?;
        tracing::info!("💾 db initialized at {}", opts.db_dir);

        let config = NodeConfig {
            prom_addr: opts.prom_address.parse().map_err(|_| Error::InvalidPromAddress)?,
            grpc_addr: opts.grpc_address.parse().map_err(|_| Error::InvalidGrpcAddress)?,
            http_listen_addr: opts.http_address.parse().map_err(|_| Error::InvalidHttpAddress)?,
            zkvm_operator: opts.operator_signer()?,
            relayer: opts.relayer_signer()?,
            db,
            exec_queue_bound: opts.exec_queue_bound,
            http_eth_rpc: opts.http_eth_rpc,
            job_manager_address: opts.job_manager_address,
            confirmations: opts.confirmations,
            job_proc_config: JobProcessorConfig {
                num_workers: opts.worker_count,
                max_retries: opts.max_retries as u32,
            },
            ws_config: WsConfig {
                ws_eth_rpc: opts.ws_eth_rpc,
                backoff_limit_ms: opts.ws_backoff_limit_ms,
                backoff_multiplier_ms: opts.ws_backoff_multiplier_ms,
            },
            job_sync_start: opts.job_sync_start,
        };

        node::run(config).await.map_err(Into::into)
    }
}
