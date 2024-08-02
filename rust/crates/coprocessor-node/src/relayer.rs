//! Logic to pull job results from the broadcast queue and write them onchain.

use std::sync::Arc;

use alloy::{
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::Address,
    providers::{PendingTransactionBuilder, ProviderBuilder},
    rpc::types::TransactionReceipt,
    signers::Signature,
    transports::http::reqwest,
};
use async_channel::Receiver;
use proto::Job;
use tokio::task::JoinHandle;
use tracing::{error, info, instrumentg};

use contracts::i_job_manager::IJobManager;

// TODO: Figure out a way to more generically represent these types without using trait objects.
type RelayerProvider = alloy::providers::fillers::FillProvider<
    alloy::providers::fillers::JoinFill<
        alloy::providers::fillers::JoinFill<
            alloy::providers::fillers::JoinFill<
                alloy::providers::fillers::JoinFill<
                    alloy::providers::Identity,
                    alloy::providers::fillers::GasFiller,
                >,
                alloy::providers::fillers::NonceFiller,
            >,
            alloy::providers::fillers::ChainIdFiller,
        >,
        alloy::providers::fillers::WalletFiller<EthereumWallet>,
    >,
    alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>>,
    alloy::transports::http::Http<reqwest::Client>,
    Ethereum,
>;

type JobManagerContract = IJobManager::IJobManagerInstance<
    alloy::transports::http::Http<reqwest::Client>,
    RelayerProvider,
>;

/// Result writer errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// failed to parse the given http rpc url
    #[error("failed to parse http_rpc_url")]
    HttpRpcUrlParse,
    /// broadcast receiver error - channel may be closed
    #[error("broadcast receiver error - channel may be closed")]
    BroadcastReceiver,
    /// rpc transport error
    #[error(transparent)]
    Rpc(#[from] alloy::transports::RpcError<alloy::transports::TransportErrorKind>),
    /// error while waiting for tx inclusion
    #[error("error while broadcasting tx: {0}")]
    TxBroadcast(alloy::contract::Error),
    /// error while waiting for tx inclusion
    #[error("error while waiting for tx inclusion: {0}")]
    TxInclusion(alloy::transports::RpcError<alloy::transports::TransportErrorKind>),
}

pub(crate) struct JobRelayer3 {
    job_manager: JobManagerContract,
}

pub(crate) struct JobRelayerBuilder<S> {
    signer: Option<S>,
}

impl<S: TxSigner<Signature> + Send + Sync + 'static> JobRelayerBuilder<S> {
    pub(crate) fn new() -> Self {
        Self { signer: None }
    }

    pub(crate) fn signer(mut self, signer: S) -> Self {
        self.signer = Some(signer);
        self
    }

    pub(crate) async fn build(
        self,
        http_rpc_url: String,
        job_manager: Address,
    ) -> Result<JobRelayer3, Error> {
        let url: reqwest::Url =
            http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse).expect("todo");
        let signer = self.signer.expect("todo");

        let address = signer.address();
        let wallet = EthereumWallet::new(signer);

        let provider =
            ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(url);
        let job_manager = JobManagerContract::new(job_manager, provider);

        Ok(JobRelayer3 { job_manager })
    }
}

impl JobRelayer3 {
    pub(crate) async fn relay(&self, job: Job) -> Result<TransactionReceipt, Error> {
        let call_builder =
            self.job_manager.submitResult(job.result.into(), job.zkvm_operator_signature.into());

        let pending_tx = call_builder.send().await.map_err(|error| {
            error!(?error, job.id, "tx broadcast failure");
            Error::TxBroadcast(error)
        })?;

        let receipt =
            pending_tx.with_required_confirmations(1).get_receipt().await.map_err(|error| {
                error!(?error, job.id, "tx inclusion failed");
                Error::TxInclusion(error)
            })?;

        info!(
            receipt.transaction_index,
            receipt.block_number,
            ?receipt.block_hash,
            ?receipt.transaction_hash,
            job.id,
            "tx included"
        );

        Ok(receipt)
    }
}

/// Pulls jobs off the broadcast queue and submits them on chain
#[derive(Debug)]
pub struct JobRelayer {
    http_rpc_url: String,
    wallet: EthereumWallet,
    job_manager_address: Address,
    broadcast_queue_receiver: Receiver<Job>,
    main_handle: Option<JoinHandle<Result<(), Error>>>,
}

impl JobRelayer {
    /// Create a new [Self] configured with the given values.
    pub fn new<S: TxSigner<Signature> + Send + Sync + 'static>(
        http_rpc_url: String,
        signer: Arc<S>,
        job_manager_address: Address,
        broadcast_queue_receiver: Receiver<Job>,
    ) -> Self {
        Self {
            http_rpc_url,
            wallet: EthereumWallet::new(signer),
            job_manager_address,
            broadcast_queue_receiver,
            main_handle: None,
        }
    }

    /// Start workers for the job result relayer subsystem.
    ///
    /// This will setup `worker_count` tasks to pull from the `broadcast_queue_receiver` and
    /// and then attempt to call `submitResult` on the `JobManager` contract at
    /// `job_manager_address`.
    #[instrument(skip_all)]
    pub async fn start(mut self) -> Result<Self, Error> {
        let url: reqwest::Url = self.http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse)?;

        let provider = ProviderBuilder::new()
            // Adds the `ChainIdFiller`, `GasFiller` and the `NonceFiller` layers.
            .with_recommended_fillers()
            .wallet(self.wallet.clone())
            .on_http(url.clone());

        let contract = IJobManager::new(self.job_manager_address, provider);
        let broadcast_queue_receiver = self.broadcast_queue_receiver.clone();

        let main_handle = tokio::spawn(async move {
            loop {
                let job =
                    broadcast_queue_receiver.recv().await.map_err(|_| Error::BroadcastReceiver)?;

                let call_builder =
                    contract.submitResult(job.result.into(), job.zkvm_operator_signature.into());

                // We broadcast sequentially to give us a better chance of not duplicating or having
                // gaps in nonces
                let tx_hash = match call_builder.send().await {
                    Ok(pending_tx) => *pending_tx.tx_hash(),
                    Err(error) => {
                        error!(?error, job.id, "tx broadcast failure");
                        continue;
                    }
                };

                // Spawn a task to track the result
                let url2 = url.clone();
                tokio::spawn(async move {
                    let provider = ProviderBuilder::new().on_http(url2);

                    let pending_tx = PendingTransactionBuilder::new(&provider, tx_hash);

                    // TODO: how do we decide on number of confirmations? Should it be configurable?
                    // https://github.com/Ethos-Works/InfinityVM/issues/130
                    let receipt =
                        match pending_tx.with_required_confirmations(1).get_receipt().await {
                            Ok(receipt) => receipt,
                            Err(error) => {
                                error!(?error, job.id, "tx inclusion failed");
                                return;
                            }
                        };

                    info!(
                        receipt.transaction_index,
                        receipt.block_number,
                        ?receipt.block_hash,
                        ?receipt.transaction_hash,
                        job.id,
                        "tx included"
                    );
                });
            }
        });

        self.main_handle = Some(main_handle);
        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, sync::Arc};

    use crate::relayer::JobRelayer;
    use alloy::{
        network::EthereumWallet,
        providers::{Provider, ProviderBuilder},
        rpc::types::Filter,
        signers::local::PrivateKeySigner,
        sol_types::SolEvent,
    };
    use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
    use test_utils::{
        anvil_with_contracts, mock_consumer_pending_job, mock_contract_input_addr, TestAnvil,
    };

    const WORKER_COUNT: usize = 3;
    const JOB_COUNT: usize = WORKER_COUNT * 10;
    const QUEUE_CAP: usize = JOB_COUNT;

    #[tokio::test]
    async fn run_can_successfully_submit_results() {
        test_utils::test_tracing();

        let TestAnvil { anvil, job_manager, relayer, coprocessor_operator, mock_consumer } =
            anvil_with_contracts().await;

        let user: PrivateKeySigner = anvil.keys()[5].clone().into();
        let user_wallet = EthereumWallet::from(user);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(user_wallet)
            .on_http(anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock_consumer, &consumer_provider);

        // 30 jobs, 30 max task in runtime, and a queue of size 30
        let (sender, receiver) = async_channel::bounded(QUEUE_CAP);

        let job_relayer = JobRelayer::new(
            anvil.endpoint().parse().unwrap(),
            Arc::new(relayer),
            job_manager,
            receiver.clone(),
        );
        job_relayer.start().await.unwrap();

        for i in 0u8..JOB_COUNT as u8 {
            let job =
                mock_consumer_pending_job(i + 1, coprocessor_operator.clone(), mock_consumer).await;

            let mock_addr = mock_contract_input_addr();
            let program_verifying_key = job.program_verifying_key.clone();

            // Create the job on chain so the contract knows to expect the result. The consumer
            // contract will create a new job
            let create_job_call =
                consumer_contract.requestBalance(program_verifying_key.into(), mock_addr);
            let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
            let log = receipt.inner.as_receipt().unwrap().logs[0]
                .log_decode::<IJobManager::JobCreated>()
                .unwrap();

            // Ensure test setup is working as we thin
            assert_eq!(job.id, log.data().jobID);

            sender.send(job).await.unwrap();
        }

        // Give the transactions some time to land
        tokio::time::sleep(tokio::time::Duration::from_millis(1_000)).await;

        // check that each job is in the anvil node logs
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();

        let seen: HashSet<_> = logs
            .into_iter()
            .map(|log| {
                let decoded = log.log_decode::<IJobManager::JobCompleted>().unwrap().data().clone();
                decoded.jobID as usize
            })
            .collect();
        // job ids from the JobManager start at 1
        let expected: HashSet<_> = (1..=JOB_COUNT).collect();

        // We expect to see exactly job ids 0 to 29 in the JobCompleted events
        assert_eq!(seen, expected);
    }
}
