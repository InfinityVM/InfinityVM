//! Logic to broadcast job result onchain.

use alloy::{
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::Address,
    providers::ProviderBuilder,
    rpc::types::TransactionReceipt,
    signers::Signature,
    transports::http::reqwest,
};
use proto::Job;
use tracing::{error, info};

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
    /// must call [`JobRelayerBuilder::signer`] before building
    #[error("ust call JobRelayerBuilder::signer before building")]
    MissingSigner,
}

/// [Builder](https://rust-unofficial.github.io/patterns/patterns/creational/builder.html) for `JobRelayer`.
#[derive(Debug)]
pub struct JobRelayerBuilder<S> {
    signer: Option<S>,
}

impl<S: TxSigner<Signature> + Send + Sync + 'static> Default for JobRelayerBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: TxSigner<Signature> + Send + Sync + 'static> JobRelayerBuilder<S> {
    /// Create a new [Self].
    pub const fn new() -> Self {
        Self { signer: None }
    }

    /// Specify the signer.
    pub fn signer(mut self, signer: S) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Build a [`JobRelayer`].
    pub fn build(self, http_rpc_url: String, job_manager: Address) -> Result<JobRelayer, Error> {
        let url: reqwest::Url = http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse)?;

        let signer = self.signer.ok_or(Error::MissingSigner)?;
        let wallet = EthereumWallet::new(signer);

        let provider =
            ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(url);
        let job_manager = JobManagerContract::new(job_manager, provider);

        Ok(JobRelayer { job_manager })
    }
}

/// Submit completed jobs onchain to the `JobManager` contract.
///
/// This is safe to use across threads and should correctly handle nonce incrementing as long as
/// no transactions fail after being broadcasted.
#[derive(Debug)]
pub struct JobRelayer {
    job_manager: JobManagerContract,
}

impl JobRelayer {
    /// Submit a completed jobs onchain to the `JobManager` contract.
    pub async fn relay(&self, job: Job) -> Result<TransactionReceipt, Error> {
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

#[cfg(test)]
mod test {
    use std::{collections::HashSet, sync::Arc};

    use crate::relayer::JobRelayerBuilder;
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
    use tokio::task::JoinSet;

    const JOB_COUNT: usize = 30;

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

        let job_relayer = JobRelayerBuilder::new()
            .signer(relayer)
            .build(anvil.endpoint().parse().unwrap(), job_manager)
            .unwrap();
        let job_relayer = Arc::new(job_relayer);
        let mut join_set = JoinSet::new();

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

            // Ensure test setup is working as we think
            assert_eq!(job.id, log.data().jobID);

            let relayer2 = Arc::clone(&job_relayer);
            join_set.spawn(async move {
                assert!(relayer2.relay(job).await.is_ok());
            });
        }

        // Wait for all the relay threads to finish so we know the transactions have landed
        while (join_set.join_next().await).is_some() {}

        // Check that each job is in the anvil node logs
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
