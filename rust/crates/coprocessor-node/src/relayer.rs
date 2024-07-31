//! Logic to pull job results from the broadcast queue and write them onchain.

use std::sync::Arc;

use alloy::{
    network::{EthereumWallet, TxSigner},
    primitives::Address,
    providers::{PendingTransactionBuilder, ProviderBuilder},
    signers::Signature,
    transports::http::reqwest,
};
use async_channel::Receiver;
use proto::Job;
use tokio::task::JoinHandle;
use tracing::{error, info, instrument, warn};

use crate::contracts::i_job_manager::IJobManager;

/// Result writer errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// failed to parse the given http rpc url
    #[error("failed to parse http_rpc_url")]
    HttpRpcUrlParse,
    /// roadcast receiver error - channel may be closed
    #[error("broadcast receiver error - channel may be closed")]
    BroadcastReceiver,
}

/// Pulls jobs off the broadcast queue and submits them on chain
#[derive(Debug)]
pub struct JobRelayer {
    http_rpc_url: String,
    wallet: EthereumWallet,
    job_manager_address: Address,
    broadcast_queue_receiver: Receiver<Job>,
    main_handle: Option<JoinHandle<Result<(), Error>>>,
    max_tokio_tasks: usize,
}

impl JobRelayer {
    /// Create a new [Self] configured with the given values.
    pub fn new<S: TxSigner<Signature> + Send + Sync + 'static>(
        http_rpc_url: String,
        signer: Arc<S>,
        job_manager_address: Address,
        broadcast_queue_receiver: Receiver<Job>,
        max_tokio_tasks: usize,
    ) -> Self {
        Self {
            http_rpc_url,
            wallet: EthereumWallet::new(signer),
            job_manager_address,
            broadcast_queue_receiver,
            main_handle: None,
            max_tokio_tasks,
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
        let max_tokio_tasks = self.max_tokio_tasks;

        let main_handle = tokio::spawn(async move {
            let metrics = tokio::runtime::Handle::current().metrics();
            loop {
                let num_alive_tasks = metrics.num_alive_tasks();
                if num_alive_tasks > max_tokio_tasks {
                    warn!(num_alive_tasks, "task count to high to broadcast");
                    tokio::task::yield_now().await;
                    continue;
                }

                let job =
                    broadcast_queue_receiver.recv().await.map_err(|_| Error::BroadcastReceiver)?;
                let call_builder =
                    contract.submitResult(job.result.into(), job.zkvm_operator_signature.into());

                // We broadcast sequentially to give us a better chance of not duplicating or having
                // gaps in nonces
                let tx_hash = match call_builder.send().await {
                    Ok(pending_tx) => *pending_tx.tx_hash(),
                    Err(error) => {
                        error!(?error, job.id, "call JobManager.submitResult failed");
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
                        match pending_tx.with_required_confirmations(2).get_receipt().await {
                            Ok(receipt) => receipt,
                            Err(error) => {
                                error!(?error, job.id, "JobManager.submitResult inclusion failed");
                                return;
                            }
                        };

                    info!(
                        receipt.transaction_index,
                        receipt.block_number,
                        ?receipt.block_hash,
                        ?receipt.transaction_hash,
                        job.id,
                        "JobManager.submitResult included in block"
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

    use crate::{
        contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer},
        relayer::JobRelayer,
        test_utils::{
            anvil_with_contracts, mock_consumer_pending_job, mock_contract_input_addr, TestAnvil,
        },
    };
    use alloy::{
        network::EthereumWallet,
        providers::{Provider, ProviderBuilder},
        rpc::types::Filter,
        signers::local::PrivateKeySigner,
        sol_types::SolEvent,
    };

    const WORKER_COUNT: usize = 3;
    const JOB_COUNT: usize = WORKER_COUNT * 10;
    const QUEUE_CAP: usize = JOB_COUNT;

    #[tokio::test]
    async fn run_can_successfully_submit_results() {
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        tracing::subscriber::set_global_default(subscriber).unwrap();

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
            QUEUE_CAP,
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
