//! Logic to broadcast job result onchain.

use alloy::{
    hex,
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::Address,
    providers::ProviderBuilder,
    rpc::types::TransactionReceipt,
    signers::Signature,
    transports::http::reqwest,
};
use std::sync::Arc;
use tracing::{error, info, instrument};

use crate::metrics::Metrics;
use contracts::i_job_manager::IJobManager;
use db::tables::{Job, RequestType};

// TODO: Figure out a way to more generically represent these types without using trait objects.
// https://github.com/Ethos-Works/InfinityVM/issues/138
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

const TX_INCLUSION_ERROR: &str = "relay_error_tx_inclusion_error";
const BROADCAST_ERROR: &str = "relay_error_broadcast_failure";

/// Relayer errors
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
    #[error("must call JobRelayerBuilder::signer before building")]
    MissingSigner,
    /// invalid job request type
    #[error("invalid job request type")]
    InvalidJobRequestType,
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
    pub fn build(
        self,
        http_rpc_url: String,
        job_manager: Address,
        confirmations: u64,
        metrics: Arc<Metrics>,
    ) -> Result<JobRelayer, Error> {
        let url: reqwest::Url = http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse)?;
        info!("🧾 relayer sending transactions to rpc url {url}");

        let signer = self.signer.ok_or(Error::MissingSigner)?;
        let wallet = EthereumWallet::new(signer);

        let provider =
            ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(url);
        let job_manager = JobManagerContract::new(job_manager, provider);

        Ok(JobRelayer { job_manager, confirmations, metrics })
    }
}

/// Submit completed jobs onchain to the `JobManager` contract.
///
/// This is safe to use across threads and should correctly handle nonce incrementing as long as
/// no transactions fail after being broadcasted.
/// TODO: <https://github.com/Ethos-Works/InfinityVM/issues/131>
#[derive(Debug)]
pub struct JobRelayer {
    job_manager: JobManagerContract,
    confirmations: u64,
    metrics: Arc<Metrics>,
}

impl JobRelayer {
    /// Submit a completed job to the `JobManager` contract for an onchain job request.
    #[instrument(skip(self, job), fields(job_id = ?job.id), err(Debug))]
    pub async fn relay_result_for_onchain_job(
        &self,
        job: Job,
    ) -> Result<TransactionReceipt, Error> {
        let call_builder = self
            .job_manager
            .submitResult(job.result_with_metadata.into(), job.zkvm_operator_signature.into());

        let pending_tx = call_builder.send().await.map_err(|error| {
            error!(?error, ?job.id, "tx broadcast failure");
            self.metrics.incr_relay_err(BROADCAST_ERROR);
            Error::TxBroadcast(error)
        })?;

        let receipt = pending_tx
            .with_required_confirmations(self.confirmations)
            .get_receipt()
            .await
            .map_err(|error| {
                error!(?error, ?job.id, "tx inclusion failed");
                self.metrics.incr_relay_err(TX_INCLUSION_ERROR);
                Error::TxInclusion(error)
            })?;

        info!(
            receipt.transaction_index,
            receipt.block_number,
            ?receipt.block_hash,
            ?receipt.transaction_hash,
            ?job.id,
            "tx included"
        );

        Ok(receipt)
    }
    /// Submit a completed job to the `JobManager` contract for an offchain job request.
    pub async fn relay_result_for_offchain_job(
        &self,
        job: Job,
        job_request_payload: Vec<u8>,
    ) -> Result<TransactionReceipt, Error> {
        if let RequestType::Offchain(request_signature) = job.request_type {
            let call_builder = self.job_manager.submitResultForOffchainJob(
                job.result_with_metadata.into(),
                job.zkvm_operator_signature.into(),
                job_request_payload.into(),
                request_signature.into(),
            );

            let pending_tx = call_builder.send().await.map_err(|error| {
                error!(
                    ?error,
                    job.nonce,
                    "tx broadcast failure: contract_address = {}",
                    hex::encode(&job.consumer_address)
                );
                self.metrics.incr_relay_err(BROADCAST_ERROR);
                Error::TxBroadcast(error)
            })?;

            let receipt =
                pending_tx.with_required_confirmations(1).get_receipt().await.map_err(|error| {
                    error!(
                        ?error,
                        job.nonce,
                        "tx inclusion failed: contract_address = {}",
                        hex::encode(&job.consumer_address)
                    );
                    self.metrics.incr_relay_err(TX_INCLUSION_ERROR);
                    Error::TxInclusion(error)
                })?;

            info!(
                receipt.transaction_index,
                receipt.block_number,
                ?receipt.block_hash,
                ?receipt.transaction_hash,
                ?job.id,
                "tx included"
            );

            Ok(receipt)
        } else {
            error!("not an offchain job request");
            Err(Error::InvalidJobRequestType)
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, sync::Arc};

    use crate::{metrics::Metrics, relayer::JobRelayerBuilder};
    use alloy::{
        network::EthereumWallet,
        primitives::{Address, U256},
        providers::{Provider, ProviderBuilder},
        rpc::types::Filter,
        signers::{local::PrivateKeySigner, Signer},
        sol_types::{SolEvent, SolValue},
    };
    use contracts::{i_job_manager::IJobManager, mock_consumer::MockConsumer};
    use db::tables::{get_job_id, Job, RequestType};
    use prometheus::Registry;
    use proto::{JobStatus, JobStatusType};
    use test_utils::{anvil_with_mock_consumer, AnvilMockConsumer, MOCK_CONTRACT_MAX_CYCLES};
    use tokio::task::JoinSet;
    use zkvm_executor::service::abi_encode_result_with_metadata;

    const JOB_COUNT: usize = 30;

    /// A mock address to use as input to the mock contract function calls
    pub(crate) fn mock_contract_input_addr() -> Address {
        Address::default()
    }

    /// Mock raw output from the zkvm program for the mock consumer contract
    pub(crate) fn mock_raw_output() -> Vec<u8> {
        (mock_contract_input_addr(), U256::default()).abi_encode()
    }

    /// Create a pending Job that has a signed result from the zkvm operator.
    ///
    /// The result here will be decodable by the `MockConsumer` contract and have
    /// a valid signature from the zkvm operator.
    pub(crate) async fn mock_consumer_pending_job(
        nonce: u8,
        operator: PrivateKeySigner,
        mock_consumer: Address,
    ) -> Job {
        let bytes = vec![nonce; 32];
        let addr = mock_contract_input_addr();
        let raw_output = mock_raw_output();

        let job_id = get_job_id(nonce.into(), mock_consumer);
        let result_with_meta = abi_encode_result_with_metadata(
            job_id,
            &addr.abi_encode(),
            MOCK_CONTRACT_MAX_CYCLES,
            &bytes,
            &raw_output,
        )
        .unwrap();
        let operator_signature =
            operator.sign_message(&result_with_meta).await.unwrap().as_bytes().to_vec();

        Job {
            id: job_id,
            nonce: 1,
            max_cycles: MOCK_CONTRACT_MAX_CYCLES,
            program_id: bytes,
            input: addr.abi_encode(),
            request_type: RequestType::Onchain,
            result_with_metadata: result_with_meta,
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
            consumer_address: mock_consumer.abi_encode(),
            zkvm_operator_signature: operator_signature,
        }
    }

    #[tokio::test]
    async fn run_can_successfully_submit_results() {
        test_utils::test_tracing();

        let AnvilMockConsumer { anvil, job_manager, relayer, coprocessor_operator, mock_consumer } =
            anvil_with_mock_consumer().await;

        let user: PrivateKeySigner = anvil.keys()[5].clone().into();
        let user_wallet = EthereumWallet::from(user);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(user_wallet)
            .on_http(anvil.endpoint().parse().unwrap());
        let consumer_contract = MockConsumer::new(mock_consumer, &consumer_provider);

        let registry = Registry::new();
        let metrics = Arc::new(Metrics::new(&registry));

        let job_relayer = JobRelayerBuilder::new()
            .signer(relayer)
            .build(anvil.endpoint().parse().unwrap(), job_manager, 1, metrics)
            .unwrap();
        let job_relayer = Arc::new(job_relayer);
        let mut join_set = JoinSet::new();

        for i in 0u8..JOB_COUNT as u8 {
            let job: db::tables::Job =
                mock_consumer_pending_job(i + 1, coprocessor_operator.clone(), mock_consumer).await;

            let mock_addr = mock_contract_input_addr();
            let program_id = job.program_id.clone();

            // Create the job on chain so the contract knows to expect the result. The consumer
            // contract will create a new job
            let create_job_call = consumer_contract.requestBalance(program_id.into(), mock_addr);
            let receipt = create_job_call.send().await.unwrap().get_receipt().await.unwrap();
            let log = receipt.inner.as_receipt().unwrap().logs[0]
                .log_decode::<IJobManager::JobCreated>()
                .unwrap();

            // Ensure test setup is working as we think
            assert_eq!(job.id, log.data().jobID);

            let relayer2 = Arc::clone(&job_relayer);
            join_set.spawn(async move {
                assert!(relayer2.relay_result_for_onchain_job(job).await.is_ok());
            });
        }

        // Wait for all the relay threads to finish so we know the transactions have landed
        while (join_set.join_next().await).is_some() {}

        // Check that each job is in the anvil node logs
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = consumer_provider.get_logs(&filter).await.unwrap();

        let seen: HashSet<[u8; 32]> = logs
            .into_iter()
            .map(|log| {
                let decoded = log.log_decode::<IJobManager::JobCompleted>().unwrap().data().clone();
                decoded.jobID.into()
            })
            .collect();
        // nonces from the consumer start at 1
        let expected: HashSet<[u8; 32]> =
            (1..=JOB_COUNT).map(|i| get_job_id(i.try_into().unwrap(), mock_consumer)).collect();

        // We expect to see exactly job ids 0 to 29 in the JobCompleted events
        assert_eq!(seen, expected);
    }
}
