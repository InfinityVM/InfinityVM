//! CLI for coprocessor-node.

use crate::{
    node::{self, NodeConfig, WsConfig},
    relayer::RelayConfig,
    MAX_DA_PER_JOB,
};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::{hex, Address},
    signers::local::LocalSigner,
};
use clap::Parser;
use k256::ecdsa::SigningKey;
use tracing::instrument;

const ENV_RELAYER_PRIV_KEY: &str = "RELAYER_PRIVATE_KEY";
const ENV_ZKVM_OPERATOR_PRIVATE_KEY: &str = "ZKVM_OPERATOR_PRIVATE_KEY";

/// Errors from the gRPC Server CLI
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// private key was not set
    #[error("environment variable {} must be set", ENV_RELAYER_PRIV_KEY)]
    RelayerPrivKeyNotSet,
    /// private key was not set
    #[error("environment variable {} must be set", ENV_ZKVM_OPERATOR_PRIVATE_KEY)]
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
    Database(#[from] ivm_db::Error),
}

type K256LocalSigner = LocalSigner<SigningKey>;

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

    /// Number of worker threads to use for processing jobs. Defaults to the number of cores - 3.
    /// We leave 2 threads for tokio and 1 thread for the DB writer.
    #[arg(long, default_value_t = 3.max(num_cpus::get_physical() - 3))]
    worker_count: usize,

    /// Max number of retries for relaying a job in the dead letter queue (DLQ) before giving up. Note
    /// that each job in the DLQ is tried once before trying again.
    #[arg(long, default_value_t = 3)]
    dlq_max_retries: usize,


    /// Max number of retries when initially attempting to relay a job. If a job fails this many times it will be added to the DLQ.
    #[arg(long, default_value_t = 1_000)]
    initial_relay_max_retries: usize,

    /// Max size for the exec queue
    #[arg(long, default_value_t = 256)]
    exec_queue_bound: usize,

    /// Block to start syncing from.
    // TODO: https://github.com/InfinityVM/InfinityVM/issues/142
    #[arg(long, default_value_t = BlockNumberOrTag::Earliest)]
    job_sync_start: BlockNumberOrTag,

    /// Required confirmations for tx
    #[arg(long, default_value_t = 1)]
    confirmations: u64,

    /// The max bytes of DA allowed per job. Defaults to the max amount of DA we can fit into a CL
    /// block.
    #[arg(long, default_value_t = MAX_DA_PER_JOB)]
    max_da_per_job: usize,
}

impl Opts {
    fn operator_signer(&self) -> Result<K256LocalSigner, Error> {
        let secret = std::env::var(ENV_ZKVM_OPERATOR_PRIVATE_KEY)
            .map_err(|_| Error::OperatorPrivKeyNotSet)?;
        Self::signer_from_hex(&secret)
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

        let db = ivm_db::init_db(&opts.db_dir)?;
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
            worker_count: opts.worker_count,
            relay_config: RelayConfig {
                confirmations: opts.confirmations,
                dlq_max_retries: opts.dlq_max_retries as u32,
                initial_relay_max_retries: opts.initial_relay_max_retries as u32,
            },
            ws_config: WsConfig {
                ws_eth_rpc: opts.ws_eth_rpc,
                backoff_limit_ms: opts.ws_backoff_limit_ms,
                backoff_multiplier_ms: opts.ws_backoff_multiplier_ms,
            },
            job_sync_start: opts.job_sync_start,
            max_da_per_job: opts.max_da_per_job,
            unsafe_skip_program_id_check: false,
        };

        node::run(config).await.map_err(Into::into)
    }
}
