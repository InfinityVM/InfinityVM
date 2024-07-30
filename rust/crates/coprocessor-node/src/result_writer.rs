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
use tracing::info;

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
            "submitResult included in block"
        );
    }
}

/// Run the the result writer subsystem.
///
/// This will setup `worker_count` tasks to pull from the `broadcast_queue_receiver` and
/// and then attempt to call `submitResult` on the `JobManager` contract at `job_manager_address`.
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
            .await
        });
    }

    Ok(set)
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::contracts::job_manager::JobManager;
    use alloy::{
        network::EthereumWallet, node_bindings::Anvil, providers::ProviderBuilder,
        signers::local::PrivateKeySigner,
    };
    use proto::Job;
    use tracing::info;
    use tracing_subscriber::EnvFilter;

    // cargo test -p coprocessor-node -- --nocapture
    #[tokio::test]
    async fn run_can_succesfully_submit_results() {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .try_init()
            .unwrap();

        info!("tracing test");
        let anvil = Anvil::new().try_spawn().unwrap();

        // Create a provider with the wallet.

        // Set up signer from the first default Anvil account (Alice).
        let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
        let wallet = EthereumWallet::from(signer.clone());
        // Create a provider with the wallet.
        let rpc_url = anvil.endpoint();

        // TODO: is there some sort of off the shelf mock provider we could inject into
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet.clone())
            .on_http(rpc_url.parse().unwrap());

        let contract = JobManager::deploy(&provider).await.unwrap();

        // Make sure the job manager is properly initialized
        // {
        //     let initial_owner =
        //         <EthereumWallet as NetworkWallet<Ethereum>>::default_signer_address(&wallet);
        //     let relayer = initial_owner.clone();
        //     let coprocessor_operator = initial_owner.clone();
        //     contract
        //         .initializeJobManager(initial_owner, relayer, coprocessor_operator)
        //         .send()
        //         .await
        //         .unwrap()
        //         .get_receipt()
        //         .await
        //         .unwrap();
        // }

        let job_manager_address = contract.address().clone();

        let (sender, receiver) = crossbeam::channel::bounded(3);

        // TODO: does this need to be wrapped in an arc?
        let signer = Arc::new(signer);
        dbg!("starting result writer");
        let mut join_set =
            super::run(rpc_url.to_string(), signer, job_manager_address, receiver.clone(), 2)
                .await
                .unwrap();

        for i in 0..10 {
            let job = Job {
                id: i,
                contract_address: vec![i as u8],
                max_cycles: i as u64,
                program_verifying_key: vec![i as u8],
                input: vec![i as u8],
                status: i as i32,
                zkvm_operator_address: vec![i as u8],
                zkvm_operator_signature: vec![i as u8],
                result: vec![i as u8],
            };
            dbg!(i);
            sender.send(job).unwrap();
            dbg!(i);
        }

        dbg!("sleeping");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // TODO: assert the chain received transactions by checking the contract state

        // We don't need to do this since it will abort threads on drop
        join_set.shutdown().await;
    }
}
