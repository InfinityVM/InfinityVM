//! Run the coprocessor node.

use crate::{
    event::{self, JobEventListener},
    gateway::{self, HttpGrpcGateway},
    intake::IntakeHandlers,
    job_processor::{JobProcessorConfig, JobProcessorService},
    metrics::{MetricServer, Metrics},
    relayer::{self, JobRelayerBuilder},
    server::CoprocessorNodeServerInner,
};
use alloy::{eips::BlockNumberOrTag, primitives::Address, signers::local::LocalSigner};
use async_channel::{bounded, Receiver, Sender};
use ivm_db::tables::Job;
use ivm_proto::coprocessor_node_server::CoprocessorNodeServer;
use ivm_zkvm_executor::service::ZkvmExecutorService;
use k256::ecdsa::SigningKey;
use prometheus::Registry;
use reth_db::Database;
use std::{
    net::{SocketAddr, SocketAddrV4},
    sync::Arc,
};
use tokio::{task::JoinHandle, try_join};
use tracing::info;

type K256LocalSigner = LocalSigner<SigningKey>;

/// Error type for this module.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// grpc server failure
    #[error("grpc server failure: {0}")]
    GrpcServer(#[from] tonic::transport::Error),
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
    /// task join error
    #[error("error handling failed")]
    ErrorHandlingFailed(#[from] tokio::task::JoinError),
    /// prometheus error
    #[error("prometheus error")]
    ErrorPrometheus(#[from] std::io::Error),
    /// http grpc gateway error
    #[error("http grpc gateway error: {0}")]
    ErrorHttpGrpcGateway(#[from] gateway::Error),
    /// job event listener error
    #[error("job event listener error: {0}")]
    JobEventListener(#[from] event::Error),
    /// relayer error
    #[error("relayer error: {0}")]
    Relayer(#[from] relayer::Error),
}

/// Configuration for ETH RPC websocket connection.
#[derive(Debug)]
pub struct WsConfig {
    /// EVM ws rpc address.
    pub ws_eth_rpc: String,
    /// Backoff limit ms.
    pub backoff_limit_ms: u64,
    /// Backoff multiplier. The sleep duration will be `num_retrys * backoff_multiplier_ms`.
    pub backoff_multiplier_ms: u64,
}

/// Arguments to run a node
#[derive(Debug)]
pub struct NodeConfig<D> {
    /// Prometheus metrics listen address.
    pub prom_addr: SocketAddr,
    /// gRPC server listen address.
    pub grpc_addr: SocketAddrV4,
    /// Address to listen on for the REST gRPC gateway.
    pub http_listen_addr: SocketAddr,
    /// zkVM operator private key.
    pub zkvm_operator: K256LocalSigner,
    /// Job result relayer private key.
    pub relayer: K256LocalSigner,
    /// Database handle
    pub db: Arc<D>,
    /// The upper bound size for the execution queue.
    pub exec_queue_bound: usize,
    /// EVM http rpc address.
    pub http_eth_rpc: String,
    /// `JobManager` contract address.
    pub job_manager_address: Address,
    /// Number of tx confirmations to wait for when submitting transactions.
    pub confirmations: u64,
    /// Job processor config values
    pub job_proc_config: JobProcessorConfig,
    /// Configuration for ETH RPC websocket connection.
    pub ws_config: WsConfig,
    /// Block number to start reading job requests from.
    pub job_sync_start: BlockNumberOrTag,
    /// The max bytes of DA per job.
    pub max_da_per_job: usize,
}

/// Run the coprocessor node.
pub async fn run<D>(
    NodeConfig {
        prom_addr,
        grpc_addr,
        http_listen_addr,
        zkvm_operator,
        relayer,
        db,
        exec_queue_bound,
        http_eth_rpc,
        job_manager_address,
        confirmations,
        job_proc_config,
        ws_config,
        job_sync_start,
        max_da_per_job,
    }: NodeConfig<D>,
) -> Result<(), Error>
where
    D: Database + 'static,
{
    info!("👷🏻 zkvm operator signer is {:?}", zkvm_operator.address());
    info!("✉️  relayer signer is {:?}", relayer.address());
    info!("📝 job manager contract address is {}", job_manager_address);

    // Setup Prometheus registry & custom metrics
    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(Metrics::new(&registry));
    let metric_server = MetricServer::new(registry.clone());

    // Start prometheus server
    let prometheus_server = tokio::spawn(async move {
        info!("📊 prometheus server listening on {}", prom_addr);
        metric_server.serve(&prom_addr.to_string()).await
    });

    // Initialize the async channels
    let (exec_queue_sender, exec_queue_receiver): (Sender<Job>, Receiver<Job>) =
        bounded(exec_queue_bound);

    // Configure the ZKVM executor
    let executor = ZkvmExecutorService::new(zkvm_operator);

    // Configure the job relayer
    let job_relayer = JobRelayerBuilder::new().signer(relayer).build(
        http_eth_rpc.clone(),
        job_manager_address,
        confirmations,
        metrics.clone(),
    )?;
    let job_relayer = Arc::new(job_relayer);

    // Configure the job processor
    let mut job_processor = JobProcessorService::new(
        Arc::clone(&db),
        exec_queue_receiver,
        job_relayer,
        executor.clone(),
        metrics,
        job_proc_config,
    );
    // Start the job processor workers
    job_processor.start().await;

    let job_event_listener = {
        let intake = IntakeHandlers::new(
            Arc::clone(&db),
            exec_queue_sender.clone(),
            executor.clone(),
            max_da_per_job,
        );
        // Configure the job listener
        let job_event_listener = JobEventListener::new(
            job_manager_address,
            intake,
            job_sync_start,
            ws_config,
            db.clone(),
        );

        // Run the job listener
        tokio::spawn(async move { job_event_listener.run().await })
    };

    let grpc_server = {
        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(ivm_proto::FILE_DESCRIPTOR_SET)
            .build_v1()
            .expect("failed to build gRPC reflection service");
        let intake = IntakeHandlers::new(
            Arc::clone(&db),
            exec_queue_sender,
            executor.clone(),
            max_da_per_job,
        );
        let coprocessor_node_server =
            CoprocessorNodeServer::new(CoprocessorNodeServerInner::new(intake));

        tokio::spawn(async move {
            info!("🚥 starting gRPC server at {}", grpc_addr);
            tonic::transport::Server::builder()
                .add_service(coprocessor_node_server)
                .add_service(reflector)
                .serve(grpc_addr.into())
                .await
        })
    };

    let http_grpc_gateway = HttpGrpcGateway::new(grpc_addr.to_string(), http_listen_addr);
    let http_grpc_gateway_server = tokio::spawn(async move { http_grpc_gateway.serve().await });

    try_join!(
        flatten(job_event_listener),
        flatten(grpc_server),
        flatten(prometheus_server),
        flatten(http_grpc_gateway_server)
    )
    .map(|_| ())
}

async fn flatten<T, E: Into<Error>>(handle: JoinHandle<Result<T, E>>) -> Result<T, Error> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(Error::ErrorHandlingFailed(err)),
    }
}
