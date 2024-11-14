//! E2E tests and helpers.
use alloy::eips::BlockNumberOrTag;
use clob_test_utils::{anvil_with_clob_consumer, AnvilClob};
use futures::future::FutureExt;
use ivm_coprocessor_node::{
    job_processor::JobProcessorConfig,
    node::{NodeConfig, WsConfig},
    MAX_DA_PER_JOB,
};
use ivm_proto::coprocessor_node_client::CoprocessorNodeClient;
use ivm_test_utils::{
    anvil_with_job_manager, get_localhost_port, sleep_until_bound, AnvilJobManager, LOCALHOST,
};
use matching_game_server::test_utils::{anvil_with_matching_game_consumer, AnvilMatchingGame};
use mock_consumer::{anvil_with_mock_consumer, AnvilMockConsumer};
use rand::Rng;
use reth_db::DatabaseEnv;
use std::{env::temp_dir, future::Future, panic::AssertUnwindSafe, sync::Arc};
use tonic::transport::Channel;

/// Arguments passed to the test function.
#[derive(Debug)]
pub struct Args {
    /// `MockConsumer` deployment.
    pub mock_consumer: Option<AnvilMockConsumer>,
    /// `ClobConsumer` deployment.
    pub clob_consumer: Option<AnvilClob>,
    /// `MatchingGameConsumer` deployment.
    pub matching_game_consumer: Option<AnvilMatchingGame>,
    /// HTTP endpoint the clob node is listening on.
    pub clob_endpoint: Option<String>,
    /// HTTP endpoint the matching game node is listening on.
    pub matching_game_endpoint: Option<String>,
    /// Anvil setup with `JobManager`.
    pub anvil: AnvilJobManager,
    /// Coprocessor Node gRPC client.
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
    /// Handle for DB. Use with care.
    pub db: Arc<DatabaseEnv>,
}

/// E2E test environment builder and runner.
#[derive(Debug, Default)]
pub struct E2E {
    clob: bool,
    matching_game: bool,
    mock_consumer: bool,
}

impl E2E {
    /// Create a new [Self].
    pub const fn new() -> Self {
        Self { clob: false, matching_game: false, mock_consumer: false }
    }

    /// Setup the clob consumer contracts and service.
    pub const fn clob(mut self) -> Self {
        self.clob = true;
        self
    }

    /// Setup the matching game consumer contracts and service.
    pub const fn matching_game(mut self) -> Self {
        self.matching_game = true;
        self
    }

    /// Setup mock consumer contract.
    pub const fn mock_consumer(mut self) -> Self {
        self.mock_consumer = true;
        self
    }
}

impl E2E {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(self, test_fn: F)
    where
        F: Fn(Args) -> R,
        R: Future<Output = ()>,
    {
        ivm_test_utils::test_tracing();

        let mut rng = rand::thread_rng();
        let test_num: u32 = rng.gen();
        let mut delete_dirs = vec![];

        let anvil_port = get_localhost_port();
        let anvil = anvil_with_job_manager(anvil_port).await;

        let http_rpc_url = anvil.anvil.endpoint();
        let ws_rpc_url = anvil.anvil.ws_endpoint();

        let coproc_db_dir = temp_dir().join(format!("infinity-coproc-test-db-{}", test_num));
        delete_dirs.push(coproc_db_dir.clone());
        let coprocessor_node_port = get_localhost_port();
        let coprocessor_node_grpc = format!("{LOCALHOST}:{coprocessor_node_port}");
        let http_port = get_localhost_port();
        let http_addr = format!("{LOCALHOST}:{http_port}");
        let prometheus_port = get_localhost_port();
        let prometheus_addr = format!("{LOCALHOST}:{prometheus_port}");
        let cn_grpc_client_url = format!("http://{coprocessor_node_grpc}");

        tracing::info!("💾 db initialized {}", coproc_db_dir.display());
        let db = ivm_db::init_db(coproc_db_dir).unwrap();
        let config = NodeConfig {
            prom_addr: prometheus_addr.parse().unwrap(),
            grpc_addr: coprocessor_node_grpc.parse().unwrap(),
            http_listen_addr: http_addr.parse().unwrap(),
            zkvm_operator: anvil.coprocessor_operator.clone(),
            relayer: anvil.relayer.clone(),
            db: Arc::clone(&db),
            exec_queue_bound: 256,
            http_eth_rpc: http_rpc_url.clone(),
            job_manager_address: anvil.job_manager,
            confirmations: 1,
            job_proc_config: JobProcessorConfig { num_workers: 2, max_retries: 1 },
            ws_config: WsConfig {
                ws_eth_rpc: ws_rpc_url.clone(),
                backoff_limit_ms: 1000,
                backoff_multiplier_ms: 3,
            },
            job_sync_start: BlockNumberOrTag::Earliest,
            max_da_per_job: MAX_DA_PER_JOB,
        };
        tokio::spawn(async move { ivm_coprocessor_node::node::run(config).await });
        sleep_until_bound(coprocessor_node_port).await;
        let coprocessor_node =
            CoprocessorNodeClient::connect(cn_grpc_client_url.clone()).await.unwrap();

        let mut args = Args {
            mock_consumer: None,
            coprocessor_node,
            anvil,
            clob_consumer: None,
            clob_endpoint: None,
            matching_game_consumer: None,
            matching_game_endpoint: None,
            db,
        };

        if self.mock_consumer {
            args.mock_consumer = Some(anvil_with_mock_consumer(&args.anvil).await)
        }

        let cn_grpc_client_url2 = cn_grpc_client_url.clone();
        if self.clob {
            let clob_consumer = anvil_with_clob_consumer(&args.anvil).await;
            let clob_db_dir = temp_dir().join(format!("infinity-clob-test-db-{}", test_num));
            delete_dirs.push(clob_db_dir.clone());
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let batcher_duration_ms = 5000;

            let clob_consumer_addr = clob_consumer.clob_consumer;
            let listen_addr2 = listen_addr.clone();
            let operator_signer = clob_consumer.clob_signer.clone();
            tokio::spawn(async move {
                clob_node::run(
                    clob_db_dir,
                    listen_addr2,
                    batcher_duration_ms,
                    operator_signer,
                    cn_grpc_client_url.clone(),
                    ws_rpc_url,
                    **clob_consumer_addr,
                    BlockNumberOrTag::Earliest,
                )
                .await
            });
            sleep_until_bound(listen_port).await;

            let clob_endpoint = format!("http://{listen_addr}");
            args.clob_endpoint = Some(clob_endpoint);
            args.clob_consumer = Some(clob_consumer);
        }

        if self.matching_game {
            let matching_game_consumer = anvil_with_matching_game_consumer(&args.anvil).await;
            let matching_game_db_dir =
                temp_dir().join(format!("infinity-matching-game-test-db-{}", test_num));
            delete_dirs.push(matching_game_db_dir.clone());
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let batcher_duration_ms = 5000;

            let matching_game_consumer_addr = matching_game_consumer.matching_game_consumer;
            let listen_addr2 = listen_addr.clone();
            let operator_signer = matching_game_consumer.matching_game_signer.clone();
            tokio::spawn(async move {
                matching_game_server::run(
                    listen_addr2,
                    batcher_duration_ms,
                    operator_signer,
                    cn_grpc_client_url2,
                    **matching_game_consumer_addr,
                )
                .await
            });
            sleep_until_bound(listen_port).await;

            let matching_game_endpoint = format!("http://{listen_addr}");
            args.matching_game_endpoint = Some(matching_game_endpoint);
            args.matching_game_consumer = Some(matching_game_consumer);
        }

        let test_result = AssertUnwindSafe(test_fn(args)).catch_unwind().await;

        for dir in delete_dirs {
            let _ = std::fs::remove_dir_all(dir);
        }

        assert!(test_result.is_ok());
    }
}
