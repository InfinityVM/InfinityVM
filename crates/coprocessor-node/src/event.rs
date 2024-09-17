//! Job request event listener.

use std::sync::Arc;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{Provider, ProviderBuilder, WsConnect},
    signers::{Signature, Signer},
    transports::{RpcError, TransportError, TransportErrorKind},
};
use contracts::job_manager::JobManager;
use db::tables::{Job, RequestType};
use proto::{JobStatus, JobStatusType};
use reth_db::Database;
use tokio::task::JoinHandle;
use tracing::error;

use crate::job_processor::JobProcessorService;

/// Errors from the job request event listener
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// event subscription error
    #[error("event subscription: {0}")]
    Subscription(#[from] TransportError<TransportErrorKind>),
    /// rpc error
    #[error("rpc: {0}")]
    Rpc(#[from] RpcError<TransportErrorKind>),
    /// job event stream unexpectedly exited
    #[error("job event stream unexpectedly exited")]
    UnexpectedExit,
}

/// Listen for job request events and push a corresponding [`Job`] onto the
/// execution queue.
pub async fn start_job_event_listener<S, D>(
    ws_rpc_url: String,
    job_manager: Address,
    job_processor: Arc<JobProcessorService<S, D>>,
    from_block: BlockNumberOrTag,
) -> Result<JoinHandle<Result<(), Error>>, Error>
where
    S: Signer<Signature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    let handle = tokio::spawn(async move {
        let mut last_seen_block = from_block.as_number().unwrap_or_default();
        let last_saved_height = job_processor.get_last_block_height().await.unwrap_or_default();
        if last_saved_height > last_seen_block {
            // update last seen block height
            last_seen_block = last_saved_height;
        }

        let ws = WsConnect::new(ws_rpc_url.clone());
        let provider = ProviderBuilder::new().on_ws(ws).await?;
        let contract = JobManager::new(job_manager, provider.clone());
        let mut latest_block = provider.get_block_number().await.unwrap();
        loop {
            // Iterate from the last seen block to the latest block
            while last_seen_block <= latest_block {
                let events = match contract
                    .JobCreated_filter()
                    .from_block(last_seen_block)
                    .to_block(last_seen_block)
                    .query()
                    .await
                {
                    Ok(events) => events,
                    Err(error) => {
                        error!(?error, "Error fetching events");
                        continue;
                    }
                };

                for (event, _) in events {
                    let job = Job {
                        id: event.jobID.into(),
                        nonce: event.nonce,
                        program_id: event.programID.to_vec(),
                        onchain_input: event.onchainInput.into(),
                        offchain_input: vec![], // Onchain jobs do not have offchain input
                        state: vec![],          // Onchain jobs are stateless
                        consumer_address: event.consumer.to_vec(),
                        max_cycles: event.maxCycles,
                        request_type: RequestType::Onchain,
                        result_with_metadata: vec![],
                        zkvm_operator_signature: vec![],
                        status: JobStatus {
                            status: JobStatusType::Pending as i32,
                            failure_reason: None,
                            retries: 0,
                        },
                    };
                    if let Err(error) = job_processor.submit_job(job).await {
                        error!(?error, "failed while submitting to job processor");
                    }
                }

                // update the last seen block height after processing the events
                last_seen_block += 1;
                if let Err(error) = job_processor.set_last_block_height(last_seen_block).await {
                    error!(?error, "failed to set last seen block height");
                }
            }

            // get latest block
            latest_block = provider.get_block_number().await.unwrap();
        }
    });

    Ok(handle)
}
