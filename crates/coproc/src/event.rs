//! Job request event listener.

use crate::{
    intake::IntakeHandlers,
    node::WsConfig,
    writer::{Write, WriterMsg},
};
use alloy::{
    eips::BlockNumberOrTag,
    hex,
    primitives::{Address, PrimitiveSignature},
    providers::{Provider, ProviderBuilder, WsConnect},
    signers::{Signer, SignerSync},
    transports::{RpcError, TransportError, TransportErrorKind},
};
use ivm_contracts::job_manager::JobManager;
use ivm_db::{
    get_last_block_height,
    tables::{Job, RequestType},
};
use ivm_proto::{JobStatus, JobStatusType, RelayStrategy};
use reth_db::Database;
use std::sync::Arc;
use tokio::{
    sync::mpsc::Sender,
    time::{sleep, Duration},
};
use tracing::{debug, error, warn};

const SUBMIT_JOB_RETRIES: usize = 4;
const SUBMIT_JOB_BACKOFF_BASE_MS: usize = 500;

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
    /// database error
    #[error("database error: {0}")]
    Database(#[from] ivm_db::Error),
}

/// Service to listen for job request events and push a corresponding [`Job`] onto the
/// execution queue.
#[derive(Debug)]
pub struct JobEventListener<S, D> {
    job_manager: Address,
    intake: IntakeHandlers<S, D>,
    from_block: BlockNumberOrTag,
    ws_config: WsConfig,
    db: Arc<D>,
    writer_tx: Sender<WriterMsg>,
}

/// Function with the same signature `JobEventListener::run`, but does not do anything.
pub async fn noop_run() -> Result<(), Error> {
    Ok(())
}

impl<S, D> JobEventListener<S, D>
where
    S: Signer<PrimitiveSignature> + SignerSync<PrimitiveSignature> + Send + Sync + Clone + 'static,
    D: Database + 'static,
{
    /// Create a new instance of [Self].
    pub const fn new(
        job_manager: Address,
        intake: IntakeHandlers<S, D>,
        from_block: BlockNumberOrTag,
        ws_config: WsConfig,
        db: Arc<D>,
        writer_tx: Sender<WriterMsg>,
    ) -> Self {
        Self { job_manager, intake, from_block, ws_config, db, writer_tx }
    }

    /// Run the job event listener
    pub async fn run(&self) -> Result<(), Error> {
        let mut last_seen_block = self.from_block.as_number().unwrap_or_default();
        let last_saved_height = self.get_last_block_height_or_0().await;
        if last_saved_height > last_seen_block {
            // update last seen block height
            last_seen_block = last_saved_height;
        }

        loop {
            let mut provider_retry = 1;
            let provider = loop {
                let ws = WsConnect::new(self.ws_config.ws_eth_rpc.clone());
                let p = ProviderBuilder::new().on_ws(ws).await;
                match p {
                    Ok(p) => break p,
                    Err(_) => {
                        let sleep_millis = provider_retry * self.ws_config.backoff_multiplier_ms;
                        sleep(Duration::from_millis(sleep_millis)).await;
                        if sleep_millis < self.ws_config.backoff_limit_ms {
                            provider_retry += 1;
                            debug!(?sleep_millis, "retrying creating ws connection to rpc");
                        } else {
                            warn!(?sleep_millis, "retrying creating ws connection to rpc");
                        }
                    }
                }
            };

            let contract = JobManager::new(self.job_manager, provider.clone());
            loop {
                // get latest block
                let latest_block = match provider.get_block_number().await {
                    Err(error) => {
                        warn!(?error, "error getting latest block number");
                        break;
                    }
                    Ok(num) => num,
                };

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
                            error!(?error, "error fetching events");
                            break;
                        }
                    };

                    for (event, _) in events {
                        let job = Job {
                            id: event.jobID.into(),
                            nonce: event.nonce,
                            program_id: event.programID.to_vec(),
                            onchain_input: event.onchainInput.into(),
                            offchain_input: vec![], // Onchain jobs do not have offchain input
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
                            relay_tx_hash: vec![],
                            blobs_sidecar: None,
                            relay_strategy: RelayStrategy::Unordered,
                        };

                        for i in 1..SUBMIT_JOB_RETRIES + 1 {
                            match self.intake.submit_job(job.clone()).await {
                                Err(error) => error!(?error, id=hex::encode(event.jobID), "failed while submitting to job processor - execution queue may be full"),
                                Ok(_) => break
                             }

                            let backoff = i * SUBMIT_JOB_BACKOFF_BASE_MS;
                            sleep(Duration::from_millis(backoff as u64)).await;
                        }
                    }

                    // update the last seen block height after processing the events
                    last_seen_block += 1;
                    self.writer_tx
                        .send((Write::LastBlockHeight(last_seen_block), None))
                        .await
                        .expect("db writer tx failed");
                }
            }
        }
    }

    /// Ergonomic helper to get the last block height from DB
    async fn get_last_block_height_or_0(&self) -> u64 {
        get_last_block_height(self.db.clone()).await.unwrap_or_default().unwrap_or_default()
    }
}
