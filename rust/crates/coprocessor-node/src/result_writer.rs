//! Logic to pull job results from the broadcast queue and write them onchain.

use std::sync::Arc;

use alloy::{
    network::{EthereumWallet, TxSigner},
    primitives::Address,
    providers::ProviderBuilder,
    signers::Signature,
};
use crossbeam::channel::Receiver;
use proto::Job;
use tokio::task::JoinSet;
use tracing::{info, info_span, instrument};

use crate::contracts::i_job_manager::IJobManager;

/// Result writer errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// job submit transaction failed
    #[error("failed to parse http_rpc_url")]
    HttpRpcUrlParse,
}

/// Task worker that pulls jobs off of the `broadcast_queue_receiver` and writes them to
/// `job_manager_address`.
async fn submit_result_worker<S>(
    http_rpc_url: String,
    signer: Arc<S>,
    job_manager_address: Address,
    broadcast_queue_receiver: Receiver<Job>,
) -> Result<(), Error>
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
    let url = http_rpc_url.parse().map_err(|_| Error::HttpRpcUrlParse)?;

    let wallet = EthereumWallet::new(signer);
    let provider = ProviderBuilder::new()
        // Adds the `ChainIdFiller`, `GasFiller` and the `NonceFiller` layers.
        .with_recommended_fillers()
        .wallet(wallet)
        // TODO: switch to IPC for performance
        .on_http(url);

    let contract = IJobManager::new(job_manager_address, provider);

    loop {
        let job = match broadcast_queue_receiver.try_recv() {
            Ok(job) => job,
            Err(_) => {
                tokio::task::yield_now().await;
                continue;
            }
        };

        let call_builder =
            contract.submitResult(job.result.into(), job.zkvm_operator_signature.into());

        let pending_tx = match call_builder.send().await {
            Ok(pending_tx) => pending_tx,
            Err(error) => {
                // TODO: what should we do with these errors
                tracing::error!(?error, job.id, "call JobManager.submitResult failed");
                continue;
            }
        };

        // TODO: how do we decide on number of confirmations? Should it be configurable?
        let receipt = match pending_tx.with_required_confirmations(2).get_receipt().await {
            Ok(receipt) => receipt,
            Err(error) => {
                tracing::error!(?error, job.id, "JobManager.submitResult inclusion failed");
                continue;
            }
        };

        info!(
            receipt.transaction_index,
            receipt.block_number,
            ?receipt.block_hash,
            ?receipt.transaction_hash,
            job.id,
            "submitResult included in block"
        );
    }
}

/// Run the the result writer subsystem.
///
/// This will setup `worker_count` tasks to pull from the `broadcast_queue_receiver` and
/// and then attempt to call `submitResult` on the `JobManager` contract at `job_manager_address`.
#[instrument(skip_all)]
pub async fn run<S>(
    http_rpc_url: String,
    signer: Arc<S>,
    job_manager_address: Address,
    broadcast_queue_receiver: Receiver<Job>,
    worker_count: u32,
) -> Result<JoinSet<Result<(), Error>>, Error>
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
    use tracing::Instrument;
    info!(worker_count, "starting result writer");

    let mut set = JoinSet::new();

    for _ in 0..worker_count {
        let http_rpc_url = http_rpc_url.clone();
        let signer = signer.clone();
        let broadcast_queue_receiver = broadcast_queue_receiver.clone();

        set.spawn(async move {
            submit_result_worker(
                http_rpc_url,
                signer,
                job_manager_address,
                broadcast_queue_receiver,
            )
            .instrument(info_span!("submit_result_worker"))
            .await
        });
    }

    Ok(set)
}

#[cfg(test)]
mod test {
    use std::{collections::HashSet, sync::Arc};

    use crate::contracts::{
        i_job_manager::IJobManager, job_manager::JobManager,
        transparent_upgradeable_proxy::TransparentUpgradeableProxy,
    };
    use alloy::{
        network::{Ethereum, EthereumWallet, NetworkWallet},
        node_bindings::Anvil,
        providers::{Provider, ProviderBuilder},
        rpc::types::Filter,
        signers::{local::PrivateKeySigner, Signer},
        sol_types::SolEvent,
    };
    use proto::Job;
    use zkvm_executor::service::abi_encode_result_with_metadata;

    /// TODO: switch to async-channel
    async fn async_send(s: crossbeam::channel::Sender<Job>, j: Job) {
        loop {
            match s.try_send(j.clone()) {
                Ok(_) => break,
                Err(_) => {
                    tokio::task::yield_now().await;
                    continue;
                }
            }
        }
    }

    // cargo test -p coprocessor-node -- --nocapture
    #[tokio::test]
    async fn run_can_successfully_submit_results() {
        // TODO: figure out test tracing subscriber
        let subscriber = tracing_subscriber::FmtSubscriber::new();
        tracing::subscriber::set_global_default(subscriber).unwrap();

        let anvil = Anvil::new().try_spawn().unwrap();

        // Set up signer from the first default Anvil account (Alice).
        let p_signer: PrivateKeySigner = anvil.keys()[0].clone().into();

        let wallet = EthereumWallet::from(p_signer.clone());
        // Create a provider with the wallet.
        let rpc_url = anvil.endpoint();
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet.clone())
            .on_http(rpc_url.parse().unwrap());

        // Deploy the JobManager implementation contract
        let job_manager_implementation = JobManager::deploy(&provider).await.unwrap();

        let initial_owner =
            <EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(&wallet);
        let relayer = initial_owner;
        let coprocessor_operator = initial_owner;

        // initializeJobManager will be called later when we deploy the proxy
        let initializer = job_manager_implementation.initializeJobManager(
            initial_owner,
            relayer,
            coprocessor_operator,
        );
        let initializer_calldata = initializer.calldata();

        // Deploy a proxy contract for JobManager
        let proxy_admin: PrivateKeySigner = anvil.keys()[1].clone().into();
        let proxy = TransparentUpgradeableProxy::deploy(
            &provider,
            *job_manager_implementation.address(),
            proxy_admin.address(),
            initializer_calldata.clone(),
        )
        .await
        .unwrap();

        let job_manager_address = *proxy.address();

        // 3 workers, 30 jobs, and a queue of size 30. That averages out to 10 txs per worker
        let (sender, receiver) = crossbeam::channel::bounded(30);

        let signer = Arc::new(p_signer);
        let _join_set = super::run(
            rpc_url.to_string(),
            signer.clone(),
            job_manager_address,
            receiver.clone(),
            3,
        )
        .await
        .unwrap();

        for i in 0u8..1 {
            // Just some deterministically generated bytes that are otherwise meaningless
            let bytes = vec![i; 32];

            // Mock a job with a valid signature from the zkvm operator
            let inputs = proto::JobInputs {
                job_id: i as u32,
                max_cycles: i as u64,
                program_verifying_key: bytes.clone(),
                program_input: bytes.clone(),
            };

            let payload = abi_encode_result_with_metadata(&inputs, &bytes);
            let operator_signature =
                signer.sign_message(&payload).await.unwrap().as_bytes().to_vec();

            let job = Job {
                id: inputs.job_id,
                max_cycles: inputs.max_cycles,
                program_verifying_key: inputs.program_verifying_key,
                input: inputs.program_input,
                result: bytes.clone(),
                status: 2,
                contract_address: bytes.clone(),
                zkvm_operator_signature: operator_signature,
                zkvm_operator_address: signer.address().to_checksum(Some(1)).as_bytes().to_vec(),
            };

            async_send(sender.clone(), job).await;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(3_000)).await;

        // check that each job is in the anvil node logs
        let filter = Filter::new().event(IJobManager::JobCompleted::SIGNATURE).from_block(0);
        let logs = provider.get_logs(&filter).await.unwrap();

        let seen: HashSet<_> = logs
            .into_iter()
            .map(|log| {
                let decoded = log.log_decode::<IJobManager::JobCompleted>().unwrap().data().clone();
                decoded.jobID
            })
            .collect();
        let expected: HashSet<_> = (0..30).collect();

        // We expect to see exactly job ids 0 to 29 in the JobCompleted events
        assert_eq!(seen, expected);
    }
}
