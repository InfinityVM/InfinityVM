//! E2E tests and helpers.
use alloy::eips::BlockNumberOrTag;
use clob_test_utils::{ivm_exec_with_clob_consumer, IvmExecClob};
use futures::future::FutureExt;
use ivm_coproc::{
    node::{NodeConfig, WsConfig},
    relayer::RelayConfig,
    MAX_DA_PER_JOB,
};
use ivm_mock_consumer::{ivm_exec_with_mock_consumer, IvmExecMockConsumer};
use ivm_proto::coprocessor_node_client::CoprocessorNodeClient;
use ivm_test_utils::{
    get_localhost_port, ivm_exec_with_job_manager, sleep_until_bound, IvmExecJobManager, LOCALHOST,
};
use matching_game_server::test_utils::{ivm_exec_with_matching_game_consumer, AnvilMatchingGame};
use rand::Rng;
use reth_db::DatabaseEnv;
use std::{env::temp_dir, future::Future, panic::AssertUnwindSafe, sync::Arc};
use tokio::runtime::Runtime;
use tonic::transport::Channel;

/// Arguments passed to the test function.
#[derive(Debug)]
pub struct Args {
    /// `MockConsumer` deployment.
    pub mock_consumer: Option<IvmExecMockConsumer>,
    /// `ClobConsumer` deployment.
    pub clob_consumer: Option<IvmExecClob>,
    /// `MatchingGameConsumer` deployment.
    pub matching_game_consumer: Option<AnvilMatchingGame>,
    /// HTTP endpoint the clob node is listening on.
    pub clob_endpoint: Option<String>,
    /// HTTP endpoint the matching game node is listening on.
    pub matching_game_endpoint: Option<String>,
    /// ivm-exec setup with `JobManager`.
    pub ivm_exec: IvmExecJobManager,
    /// Coprocessor Node gRPC client.
    pub coprocessor_node: CoprocessorNodeClient<Channel>,
    /// Coprocessor Node gRPC endpoint.
    pub coprocessor_node_endpoint: String,
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

        let ivm_exec_port = get_localhost_port();
        let ivm_exec = ivm_exec_with_job_manager(ivm_exec_port, None).await;

        let http_rpc_url = ivm_exec.ivm_exec.endpoint();
        let ws_rpc_url = ivm_exec.ivm_exec.ws_endpoint();

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
            zkvm_operator: ivm_exec.coprocessor_operator.clone(),
            relayer: ivm_exec.relayer.clone(),
            db: Arc::clone(&db),
            exec_queue_bound: 256,
            http_eth_rpc: http_rpc_url.clone(),
            job_manager_address: ivm_exec.job_manager,
            worker_count: 4,
            relay_config: RelayConfig {
                confirmations: 1,
                dlq_max_retries: 0,
                initial_relay_max_retries: 1,
            },
            ws_config: WsConfig {
                ws_eth_rpc: ws_rpc_url.clone(),
                backoff_limit_ms: 1000,
                backoff_multiplier_ms: 3,
            },
            job_sync_start: BlockNumberOrTag::Earliest,
            max_da_per_job: MAX_DA_PER_JOB,
            unsafe_skip_program_id_check: true,
            disable_events: false,
        };
        tokio::spawn(async move { ivm_coproc::node::run(config).await });
        sleep_until_bound(coprocessor_node_port).await;
        let coprocessor_node =
            CoprocessorNodeClient::connect(cn_grpc_client_url.clone()).await.unwrap();

        let mut args = Args {
            mock_consumer: None,
            coprocessor_node,
            coprocessor_node_endpoint: cn_grpc_client_url.clone(),
            ivm_exec,
            clob_consumer: None,
            clob_endpoint: None,
            matching_game_consumer: None,
            matching_game_endpoint: None,
            db,
        };

        if self.mock_consumer {
            args.mock_consumer = Some(ivm_exec_with_mock_consumer(&args.ivm_exec).await)
        }

        let cn_grpc_client_url2 = cn_grpc_client_url.clone();
        if self.clob {
            let clob_consumer = ivm_exec_with_clob_consumer(&args.ivm_exec).await;
            let clob_db_dir = temp_dir().join(format!("infinity-clob-test-db-{}", test_num));
            delete_dirs.push(clob_db_dir.clone());
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let batcher_duration_ms = 1000;

            let clob_consumer_addr = clob_consumer.clob_consumer;
            let listen_addr2 = listen_addr.clone();
            let operator_signer = clob_consumer.clob_signer.clone();

            std::thread::spawn(move || {
                Runtime::new()
                    .unwrap()
                    .block_on(async move {
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
                    })
                    .unwrap();
            });
            sleep_until_bound(listen_port).await;

            let clob_endpoint = format!("http://{listen_addr}");
            args.clob_endpoint = Some(clob_endpoint);
            args.clob_consumer = Some(clob_consumer);
        }

        if self.matching_game {
            let matching_game_consumer = ivm_exec_with_matching_game_consumer(&args.ivm_exec).await;
            let matching_game_db_dir =
                temp_dir().join(format!("infinity-matching-game-test-db-{}", test_num));
            delete_dirs.push(matching_game_db_dir.clone());
            let listen_port = get_localhost_port();
            let listen_addr = format!("{LOCALHOST}:{listen_port}");
            let batcher_duration_ms = 1000;

            let matching_game_consumer_addr = matching_game_consumer.matching_game_consumer;
            let listen_addr2 = listen_addr.clone();
            let operator_signer = matching_game_consumer.matching_game_signer.clone();
            std::thread::spawn(move || {
                Runtime::new().unwrap().block_on(async move {
                    matching_game_server::run(
                        listen_addr2,
                        batcher_duration_ms,
                        operator_signer,
                        cn_grpc_client_url2,
                        **matching_game_consumer_addr,
                    )
                    .await
                })
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
