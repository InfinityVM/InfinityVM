//! Job request event listener.

use std::sync::Arc;

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
    signers::{Signature, Signer},
    transports::{RpcError, TransportError, TransportErrorKind},
};
use contracts::job_manager::JobManager;
use db::tables::{Job, RequestType};
use futures_util::StreamExt;
use proto::{JobStatus, JobStatusType};
use reth_db::Database;
use tokio::{
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{error, warn};

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
        let mut last_seen_block = from_block;
        let mut retry = 1;
        let ws = WsConnect::new(ws_rpc_url.clone());
        let provider = ProviderBuilder::new().on_ws(ws).await?;
        let contract = JobManager::new(job_manager, provider);
        loop {
            // We have this loop so we can recreate a subscription stream in case any issue is
            // encountered
            let sub =
                match contract.JobCreated_filter().from_block(last_seen_block).subscribe().await {
                    Ok(sub) => sub,
                    Err(error) => {
                        error!(?error, "attempted to create websocket subscription");
                        continue;
                    }
                };
            let mut stream = sub.into_stream();

            while let Some(event) = stream.next().await {
                let (event, log) = match event {
                    Ok((event, log)) => (event, log),
                    Err(error) => {
                        error!(?error, "event listener");
                        continue;
                    }
                };

                let job = Job {
                    id: event.jobID.into(),
                    nonce: event.nonce,
                    program_id: event.programID.clone().to_vec(),
                    input: event.programInput.into(),
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
                if let Some(n) = log.block_number {
                    last_seen_block = BlockNumberOrTag::Number(n);
                }
            }

            sleep(Duration::from_millis(retry * 10)).await;
            warn!(?retry, ?last_seen_block, "websocket reconnecting");
            retry += 1;
        }
    });

    Ok(handle)
}
