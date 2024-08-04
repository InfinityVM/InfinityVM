//! Job request event listener.

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
};
use async_channel::Sender;
use contracts::job_manager::JobManager;
// use futures_util::StreamExt;
use alloy::transports::{RpcError, TransportError, TransportErrorKind};
use futures_util::StreamExt;
use proto::{Job, JobStatus};
use tokio::task::JoinHandle;
use tracing::error;

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
    /// 
    #[error("job event stream unexpectedly exited")]
    UnexpectedExit,
}

/// Listen for job request events and push a corresponding [`proto::Job`] onto the
/// execution queue.
pub async fn start_job_event_listener(
    http_rpc_url: String,
    job_manager: Address,
    exec_queue_sender: Sender<Job>,
    from_block: BlockNumberOrTag,
) -> Result<JoinHandle<Result<(), Error>>, Error> {
    let ws = WsConnect::new(http_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let contract = JobManager::new(job_manager, provider);
    let sub = contract.JobCreated_filter().from_block(from_block).subscribe().await?;
    let mut stream = sub.into_stream();

    let handle = tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            let (event, log) = match event {
                Ok((event, log)) => (event, log),
                Err(error) => {
                    error!(?error, "event listener");
                    continue;
                }
            };

            let job = Job {
                id: event.jobID,
                program_verifying_key: event.programID.to_vec(),
                input: event.programInput.into(),
                contract_address: log.address().to_vec(),
                max_cycles: event.maxCycles,
                result: vec![],
                zkvm_operator_address: vec![],
                zkvm_operator_signature: vec![],
                status: JobStatus::Pending.into(),
            };

            if let Err(error) = exec_queue_sender.send(job).await {
                error!(?error, "please report: error sending job to execution queue");
            }
        }

        error!("please report: job created event listener unexpectedly exited early");
        Err(Error::UnexpectedExit)
    });

    Ok(handle)
}
