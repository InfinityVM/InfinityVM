//! Run the coprocessor node.

use crate::{
    config,
    event::{self, JobEventListener},
    gateway::{self, HttpGrpcGateway},
    intake::{IntakeConfig, IntakeHandlers},
    job_executor::{JobExecutor, JobExecutorConfig},
    metrics::{MetricServer, Metrics},
    relayer::{self, JobRelayerBuilder, RelayConfig, RelayCoordinator},
    server::CoprocessorNodeServerInner,
    writer::{self, Write, Writer},
};
use alloy::{eips::BlockNumberOrTag, primitives::Address, signers::local::LocalSigner};
use ivm_proto::coprocessor_node_server::CoprocessorNodeServer;
use ivm_zkvm_executor::service::ZkvmExecutorService;
use k256::ecdsa::SigningKey;
use prometheus::Registry;
use reth_db::Database;
use std::{
    any::Any,
    net::{SocketAddr, SocketAddrV4},
    sync::Arc,
    time::Duration,
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
    /// writer error
    #[error("writer error: {0}")]
    Writer(#[from] writer::Error),
    /// thread join error
    #[error("thread join error: ")]
    StdJoin(Box<dyn Any + Send + 'static>),
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
    pub worker_count: usize,
    /// Job processor config values
    pub relay_config: RelayConfig,
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
        worker_count,
        relay_config,
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
    let (exec_queue_sender, exec_queue_receiver) = flume::bounded(exec_queue_bound);
    // Initialize the writer channel
    let (writer_tx, writer_rx) = flume::bounded(exec_queue_bound * 4);
    // Initialize channel to relay coordinator
    let (relay_tx, relay_rx) = flume::bounded(exec_queue_bound * 4);

    // Configure the ZKVM executor
    let executor = ZkvmExecutorService::new(zkvm_operator);

    let writer = Writer::new(Arc::clone(&db), writer_rx);
    let writer_handle = std::thread::spawn(move || writer.start_blocking());

    // Configure the job relayer
    let job_relayer = JobRelayerBuilder::new().signer(relayer).build(
        http_eth_rpc.clone(),
        job_manager_address,
        relay_config.confirmations,
        metrics.clone(),
    )?;
    let job_relayer = Arc::new(job_relayer);

    let relay_coordinator = {
        let relay_coordinator = RelayCoordinator::new(
            writer_tx.clone(),
            relay_rx,
            job_relayer.clone(),
            db.clone(),
            relay_config.max_retries,
            metrics.clone(),
        );
        tokio::spawn(async move { relay_coordinator.start().await })
    };

    // Configure the job processor
    let job_executor = JobExecutor::new(
        Arc::clone(&db),
        exec_queue_receiver,
        executor.clone(),
        JobExecutorConfig {
            metrics,
            num_workers: worker_count,
            writer_tx: writer_tx.clone(),
            relay_tx: relay_tx.clone(),
            config: config::Config::default(),
        },
    );
    // Start the job processor workers
    job_executor.start().await;

    let intake = IntakeHandlers::new(
        Arc::clone(&db),
        exec_queue_sender.clone(),
        executor.clone(),
        IntakeConfig {
            max_da_per_job,
            writer_tx: writer_tx.clone(),
            relay_tx: relay_tx.clone(),
            config: config::Config::default(),
        },
    );

    let job_event_listener = {
        // Configure the job listener
        let job_event_listener = JobEventListener::new(
            job_manager_address,
            intake.clone(),
            job_sync_start,
            ws_config,
            db.clone(),
            writer_tx.clone(),
        );

        // Run the job listener
        tokio::spawn(async move { job_event_listener.run().await })
    };

    let grpc_server = {
        let reflector = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(ivm_proto::FILE_DESCRIPTOR_SET)
            .build_v1()
            .expect("failed to build gRPC reflection service");

        let coprocessor_node_server =
            CoprocessorNodeServer::new(CoprocessorNodeServerInner::new(intake.clone()));

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

    // Check std thread handles to ensure any errors are bubbled up
    let threads = tokio::spawn(async {
        loop {
            if writer_handle.is_finished() {
                return writer_handle.join().map_err(Error::StdJoin);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });

    let result = try_join!(
        flatten(job_event_listener),
        flatten(grpc_server),
        flatten(prometheus_server),
        flatten(http_grpc_gateway_server),
        flatten(relay_coordinator),
        flatten(threads)
    )
    .map(|_| ());

    // We need to manually shutdown any standard threads
    let _ = writer_tx.send_async((Write::Kill, None)).await;

    result
}

async fn flatten<T, E: Into<Error>>(handle: JoinHandle<Result<T, E>>) -> Result<T, Error> {
    match handle.await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(Error::ErrorHandlingFailed(err)),
    }
}
