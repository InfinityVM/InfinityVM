//! Integration tests and helpers.
use futures::future::FutureExt;
use rand::Rng;
use std::{
    future::Future,
    net::TcpListener,
    panic::AssertUnwindSafe,
    process::{self, Command},
    thread,
    time::Duration,
};
use tonic::transport::Channel;

use proto::zkvm_executor_client::ZkvmExecutorClient;

const LOCALHOST: &str = "127.0.0.1";
const EXECUTOR_DEBUG_BIN: &str = "../target/debug/executor";

/// Kill [`std::process::Child`] on `drop`
#[derive(Debug)]
pub struct ProcKill(process::Child);

impl From<process::Child> for ProcKill {
    fn from(child: process::Child) -> Self {
        Self(child)
    }
}

impl Drop for ProcKill {
    fn drop(&mut self) {
        drop(self.0.kill());
    }
}

/// gRPC clients
#[derive(Debug)]
pub struct Clients {
    /// zkvm executor gRPC client. Connected to the
    pub executor: ZkvmExecutorClient<Channel>,
}

/// Integration test environment builder and runner.
#[derive(Debug)]
pub struct Integration;

impl Integration {
    /// Run the given `test_fn`.
    pub async fn run<F, R>(test_fn: F)
    where
        F: Fn(Clients) -> R,
        R: Future<Output = ()>,
    {
        // Start executor
        let db_dir = tempfile::Builder::new().prefix("zkvm-executor-test-db").tempdir().unwrap();
        let executor_port = get_localhost_port();
        let _proc: ProcKill = Command::new(EXECUTOR_DEBUG_BIN)
            .arg("--ip")
            .arg(LOCALHOST)
            .arg("--port")
            .arg(executor_port.to_string())
            .arg("--db-dir")
            .arg(db_dir.path())
            .arg("dev")
            .spawn()
            .unwrap()
            .into();

        sleep_until_bound(executor_port);
        let executor = ZkvmExecutorClient::connect(format!("http://{LOCALHOST}:{executor_port}"))
            .await
            .unwrap();

        let clients = Clients { executor };

        let test_result = AssertUnwindSafe(test_fn(clients)).catch_unwind().await;
        assert!(test_result.is_ok())
    }
}

fn get_localhost_port() -> u16 {
    let mut rng = rand::thread_rng();

    for _ in 0..64 {
        let port = rng.gen_range(49152..65535);
        if TcpListener::bind((LOCALHOST, port)).is_ok() {
            return port;
        }
    }

    panic!("no port found after 64 attempts");
}

fn sleep_until_bound(port: u16) {
    for _ in 0..16 {
        if TcpListener::bind((LOCALHOST, port)).is_err() {
            return;
        }

        thread::sleep(Duration::from_secs(1));
    }

    panic!("localhost:{port} was not successfully bound");
}
