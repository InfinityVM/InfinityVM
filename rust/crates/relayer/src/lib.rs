use proto::Job;
use std::{
    fmt::Debug,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
    thread,
    time::Duration,
    fs,
    env,
};
use url::Url;
use alloy::{
    contract::{ContractInstance, Interface},
    dyn_abi::DynSolValue,
    network::{Ethereum, TransactionBuilder, EthereumWallet},
    primitives::{Address, hex, U256},
    providers::{Provider, ProviderBuilder, WsConnect, ReqwestProvider},
    rpc::types::TransactionRequest,
    signers::local::PrivateKeySigner,
    transports::http::{Client, Http},
    sol,
};
use alloy::providers::fillers::{FillProvider, JoinFill, RecommendedFiller, WalletFiller};

sol!(
    #[sol(rpc)]
    JobManager,
    "../../../contracts/out/JobManager.sol/JobManager.json"
);
use tokio;

use crossbeam::queue::ArrayQueue;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use crate::JobManager::JobManagerInstance;

#[derive(Debug)]
pub enum Error {
    EthClientError,
}
#[async_trait]
pub trait EthClient: Debug + Send + Sync + 'static{
    async fn execute_callback<'a>(&'a self, job: &'a Job) -> Result<(), &'a str> ;
}

#[derive(Debug)]
pub struct MockEthClient {
    pub times_called: Arc<Mutex<i32>>,
}

#[derive(Debug)]
pub struct RpcEthClient {
    // client: Arc<ReqwestProvider>,
    // wallet: EthereumWallet,
    contract: JobManagerInstance<Http<Client>, FillProvider<JoinFill<RecommendedFiller, WalletFiller<EthereumWallet>>, ReqwestProvider, Http<Client>, Ethereum>>,
}

#[async_trait]
impl EthClient for MockEthClient {
    async fn execute_callback<'a>(&'a self, job: &'a Job) -> Result<(), &'a str>  {

        let mut times_called = self.times_called.lock().unwrap();
        *times_called += 1;
        println!("Executing callback for job: {}", job.id);
        // Ok(());
       todo!()
    }
}
impl MockEthClient {
    fn new(
        eth_http_url: &str,
        private_key: &str,
        contract_address: &str) -> Result<Self, Box<dyn std::error::Error>> {
        todo!()
    }
}

#[async_trait]
impl EthClient for RpcEthClient {

    async fn execute_callback<'a>(&'a self, job: &'a Job) -> Result<(), &'a str> {
       Ok(())
    }
}

impl RpcEthClient {
    async fn new(
        eth_rpc_url: &str,
        private_key: &str,
        contract_address: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // env::var("RELAYER_PRIVATE_KEY").expect("relayer private key is not set");
        let signer: PrivateKeySigner = private_key.parse().expect("invalid private key");
        let wallet = EthereumWallet::from(signer.clone());

        let rpc_url = Url::parse(&eth_rpc_url).expect("invalid RPC url");

        let client   = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(rpc_url);

        println!("provider: {:?}", client);

        let chain_id_connected = client.get_chain_id().await?;
        println!("connected to chain id: {}", chain_id_connected);

        let contract_address = contract_address.parse().expect("invalid JobManager contract address");
        let contract= JobManagerInstance::new(contract_address, client);

        // contract.submitResult().await.unwrap()
        Ok(RpcEthClient{
            contract,
        })
    }
}
#[derive(Debug)]
pub struct Relayer {
    pub eth_client: Arc<dyn EthClient>,
    pub broadcast_queue: Arc<ArrayQueue<Job>>,
    pub stop_flag: Arc<AtomicBool>,
}

impl Relayer {
    pub fn new(eth_client: Arc<dyn EthClient>, broadcast_queue: Arc<ArrayQueue<Job>>) -> Self {
        Relayer {
            eth_client,
            broadcast_queue,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), Error> {
        println!("Starting Relayer");

        while !self.stop_flag.load(Ordering::SeqCst) {
            let finished_job = self.broadcast_queue.pop();

            match finished_job {
                Some(job) => match self.eth_client.execute_callback(&job).await {
                    Ok(_) => println!("Executed callback against job: {}", job.id),
                    Err(e) => {
                        println!("Error executing callback {}", e);
                        return Err(Error::EthClientError);
                    }
                },
                None => {
                    println!("No jobs to process");
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }

        println!("Relayer stopped");
        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_relayer() {
        let mock_eth_client = Arc::new(MockEthClient {
            times_called: Arc::new(Mutex::new(0)),
        });

        let broadcast_queue = Arc::new(ArrayQueue::new(10));
        let relayer = Arc::new(Relayer::new(mock_eth_client.clone(), broadcast_queue.clone()));

        for i in 0..3 {
            let job = Job {
                id: i,
                program_verifying_key: vec![0x01, 0x02, 0x03],
                vm_type: 0,
                input: vec![0x04, 0x05, 0x06],
                contract_address: vec![0x07, 0x08, 0x09],
                max_cycles: 1000,
                result: vec![],
                zkvm_operator_address: vec![0x0A, 0x0B, 0x0C],
                zkvm_operator_signature: vec![0x0D, 0x0E, 0x0F],
                status: 1,
            };
            relayer.broadcast_queue.push(job).unwrap();
        }

        let relayer_handle = {
            let relayer = relayer.clone();
            thread::spawn(move || {
                relayer.start().unwrap();
            })
        };

        sleep(Duration::from_secs(1)).await;
        relayer.stop();
        relayer_handle.join().unwrap();

        assert_eq!(*mock_eth_client.times_called.lock().unwrap(), 3);
        assert_eq!(relayer.broadcast_queue.len(), 0);
    }
}