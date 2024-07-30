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
use tracing::{info, trace};

use crate::contracts::job_manager::IJobManager;

/// Chain writer errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// job submit transaction failed
    #[error("failed to parse http_rpc_url")]
    HttpRpcUrlParse,
}

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
                continue
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
            "submitResult included in block"
        );
    }
}

/// Run the the result writer subsystem.
///
/// This will setup [`worker_count`] tasks to pull from the [`broadcast_queue_receiver`] and
/// and then attempt to call `submitResult` on the `JobManager` contract at [`job_manager_address`].
pub async fn run<S>(
    http_rpc_url: String,
    signer: Arc<S>,
    job_manager_address: Address,
    broadcast_queue_receiver: Receiver<Job>,
    worker_count: u32,
) -> Result<(), Error>
where
    S: TxSigner<Signature> + Send + Sync + 'static,
{
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
            .await
        });
    }

    while let Some(res) = set.join_next().await {
        trace!(?res, "submit result worker task exited");
    }

    Ok(())
}
