//! Locally spin up the coprocessor node, anvil, and the clob.

use alloy::primitives::hex;
use clob_node::{
    CLOB_BATCHER_DURATION_MS, CLOB_CN_GRPC_ADDR, CLOB_CONSUMER_ADDR, CLOB_DB_DIR, CLOB_ETH_WS_ADDR,
    CLOB_JOB_SYNC_START, CLOB_LISTEN_ADDR, CLOB_OPERATOR_KEY,
};
use clob_programs::CLOB_ELF;
use clob_test_utils::{anvil_with_clob_consumer, mint_and_approve};
use contracts::{DeployInfo, DEFAULT_DEPLOY_INFO};
use intensity_test_methods::INTENSITY_TEST_GUEST_ELF;
use mock_consumer::anvil_with_mock_consumer;
use mock_consumer_methods::MOCK_CONSUMER_GUEST_ELF;
use mock_consumer_sp1::SP1_MOCK_CONSUMER_GUEST_ELF;
use proto::{coprocessor_node_client::CoprocessorNodeClient, SubmitProgramRequest, VmType};
use std::{fs::File, process::Command};
use test_utils::{anvil_with_job_manager, sleep_until_bound_config, ProcKill, LOCALHOST};
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

const ANVIL_PORT: u16 = 8545;
const COPROCESSOR_GRPC_PORT: u16 = 50420;
const COPROCESSOR_PROM_PORT: u16 = 50069;
const HTTP_GATEWAY_LISTEN_PORT: u16 = 8080;
const CLOB_PORT: u16 = 40420;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    info!("Starting anvil on port: {ANVIL_PORT}");
    let job_manager_deploy = anvil_with_job_manager(ANVIL_PORT).await;
    sleep_until_bound_config(ANVIL_PORT, 30).await.unwrap();

    let job_manager = job_manager_deploy.job_manager.to_string();
    let chain_id = job_manager_deploy.anvil.chain_id().to_string();
    let http_rpc_url = job_manager_deploy.anvil.endpoint();
    let ws_rpc_url = job_manager_deploy.anvil.ws_endpoint();

    let coproc_grpc = format!("{LOCALHOST}:{COPROCESSOR_GRPC_PORT}");
    let http_gateway_listen_address = format!("{LOCALHOST}:{HTTP_GATEWAY_LISTEN_PORT}");
    let coproc_prometheus = format!("{LOCALHOST}:{COPROCESSOR_PROM_PORT}");
    let coproc_relayer_private = hex::encode(job_manager_deploy.relayer.to_bytes());
    let coproc_operator_private = hex::encode(job_manager_deploy.coprocessor_operator.to_bytes());
    let coproc_db_dir =
        tempfile::Builder::new().prefix("infinity-coproc-local-db").tempdir().unwrap();
    std::fs::create_dir_all("./logs").unwrap();
    let coproc_log_file = "./logs/coprocessor_node.log".to_string();
    info!(?coproc_log_file, "Writing coprocessor logs to: ");
    let coproc_logs = File::create(coproc_log_file).expect("failed to open log");
    let coproc_logs2 = coproc_logs.try_clone().unwrap();
    info!("Starting coprocessor node");
    info!(?coproc_prometheus, ?coproc_grpc, "Coprocessor listening on");
    info!(?coproc_relayer_private, ?coproc_operator_private, "Coprocessor keys");
    info!(?http_gateway_listen_address, "REST gRPC gateway listening on");

    let proc: ProcKill = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("coprocessor-node")
        .arg("--")
        .env("RELAYER_PRIVATE_KEY", coproc_relayer_private)
        .env("ZKVM_OPERATOR_PRIV_KEY", coproc_operator_private)
        .env("RUST_LOG_FILE", "coproc_tracing.log")
        .env("RUST_LOG_DIR", "./logs")
        .env("RUST_LOG_FORMAT", "text")
        .arg("--grpc-address")
        .arg(&coproc_grpc)
        .arg("--http-address")
        .arg(&http_gateway_listen_address)
        .arg("--prom-address")
        .arg(&coproc_prometheus)
        .arg("--http-eth-rpc")
        .arg(&http_rpc_url)
        .arg("--ws-eth-rpc")
        .arg(&ws_rpc_url)
        .arg("--job-manager-address")
        .arg(job_manager)
        .arg("--chain-id")
        .arg(chain_id)
        .arg("--db-dir")
        .arg(coproc_db_dir.path())
        .arg("--worker-count")
        .arg("32")
        .stdout(coproc_logs)
        .stderr(coproc_logs2)
        .spawn()
        .unwrap()
        .into();
    sleep_until_bound_config(COPROCESSOR_GRPC_PORT, 5 * 60).await.unwrap();
    info!("coproc-node process ID: {}", proc.0.id());

    info!("Deploying MockConsumer contract");
    let mock_consumer = anvil_with_mock_consumer(&job_manager_deploy).await;
    let mock_consumer_addr = mock_consumer.mock_consumer.to_string();
    info!(?mock_consumer_addr, "MockConsumer deployed at");

    let clob_deploy = anvil_with_clob_consumer(&job_manager_deploy).await;
    let clob_db_dir = tempfile::Builder::new().prefix("infinity-clob-local-db").tempdir().unwrap();
    let clob_http = format!("{LOCALHOST}:{CLOB_PORT}");
    let batcher_duration_ms = 1000;
    let clob_log_file = "./logs/clob_node.log".to_string();
    let clob_logs = File::create(clob_log_file.clone()).expect("failed to open log");
    let clob_logs2 = clob_logs.try_clone().unwrap();
    let clob_consumer_addr = clob_deploy.clob_consumer.to_string();
    let clob_operator = hex::encode(clob_deploy.clob_signer.to_bytes());
    let client_coproc_grpc = format!("http://{LOCALHOST}:{COPROCESSOR_GRPC_PORT}");

    let accounts_num = 100;
    info!("Minting tokens to the first {} accounts", accounts_num);
    mint_and_approve(&clob_deploy, http_rpc_url.clone(), accounts_num).await;

    {
        let deploy_info = DeployInfo {
            job_manager: job_manager_deploy.job_manager,
            quote_erc20: clob_deploy.quote_erc20,
            base_erc20: clob_deploy.base_erc20,
            clob_consumer: clob_deploy.clob_consumer,
            mock_consumer: mock_consumer.mock_consumer,
        };

        let filename = DEFAULT_DEPLOY_INFO.to_string();
        let json = serde_json::to_string_pretty(&deploy_info).unwrap();

        info!("DeployInfo: {}", json);
        info!("Writing deploy info to: {}", filename);
        std::fs::write(filename, json).unwrap();
    }

    {
        let mut coproc_client =
            CoprocessorNodeClient::connect(client_coproc_grpc.clone()).await.unwrap();
        info!("Submitting CLOB ELF to coprocessor node");
        let submit_program_request =
            SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into() };
        coproc_client.submit_program(submit_program_request).await.unwrap();

        info!("Submitting MockConsumer ELF to coprocessor node");
        let submit_program_request = SubmitProgramRequest {
            program_elf: MOCK_CONSUMER_GUEST_ELF.to_vec(),
            vm_type: VmType::Risc0.into(),
        };
        coproc_client.submit_program(submit_program_request).await.unwrap();

        info!("Submitting SP1 MockConsumer ELF to coprocessor node");
        let submit_program_request = SubmitProgramRequest {
            program_elf: SP1_MOCK_CONSUMER_GUEST_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
        };
        coproc_client.submit_program(submit_program_request).await.unwrap();

        info!("Submitting IntensityTest ELF to coprocessor node");
        let submit_program_request = SubmitProgramRequest {
            program_elf: INTENSITY_TEST_GUEST_ELF.to_vec(),
            vm_type: VmType::Risc0.into(),
        };
        coproc_client.submit_program(submit_program_request).await.unwrap();
    }

    info!("Starting CLOB");
    info!(?clob_http, "CLOB listening on");
    info!(?clob_log_file, "CLOB logs");
    let _proc: ProcKill = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("clob-node")
        .env(CLOB_LISTEN_ADDR, clob_http)
        .env(CLOB_DB_DIR, clob_db_dir.path())
        .env(CLOB_CN_GRPC_ADDR, client_coproc_grpc)
        .env(CLOB_ETH_WS_ADDR, ws_rpc_url)
        .env(CLOB_CONSUMER_ADDR, clob_consumer_addr)
        .env(CLOB_BATCHER_DURATION_MS, batcher_duration_ms.to_string())
        .env(CLOB_OPERATOR_KEY, clob_operator)
        .env(CLOB_JOB_SYNC_START, "0x0")
        .stdout(clob_logs)
        .stderr(clob_logs2)
        .spawn()
        .unwrap()
        .into();
    sleep_until_bound_config(CLOB_PORT, 5 * 60).await.unwrap();

    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
    tokio::select! {
        _ = signal_terminate.recv() => info!("Received SIGTERM."),
        _ = signal_interrupt.recv() => info!("Received SIGINT."),
    };
}
