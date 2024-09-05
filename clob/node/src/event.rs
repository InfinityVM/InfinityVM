//! Deposit event listener.

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
    signers::{Signature, Signer},
    transports::{RpcError, TransportError, TransportErrorKind},
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::{ApiResponse, Request};
use futures_util::StreamExt;
use reth_db::Database;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{error, warn};

/// Errors from the deposit event listener
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// event subscription error
    #[error("event subscription: {0}")]
    Subscription(#[from] TransportError<TransportErrorKind>),
    /// rpc error
    #[error("rpc: {0}")]
    Rpc(#[from] RpcError<TransportErrorKind>),
    /// deposit event stream unexpectedly exited
    #[error("deposit event stream unexpectedly exited")]
    UnexpectedExit,
}

/// Listen for deposit events and push a corresponding
/// `Deposit` request
pub async fn start_deposit_event_listener(
    ws_rpc_url: String,
    clob_consumer: Address,
    event_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    from_block: BlockNumberOrTag,
) {
    let mut last_seen_block = from_block;
    let mut retry = 1;
    let ws = WsConnect::new(ws_rpc_url.clone());
    let provider = ProviderBuilder::new().on_ws(ws).await.unwrap();
    let contract = ClobConsumer::new(clob_consumer, &provider);
    loop {
        // We have this loop so we can recreate a subscription stream in case any issue is
        // encountered
        let sub = match contract.Deposit_filter().from_block(last_seen_block).subscribe().await {
            Ok(sub) => sub,
            Err(_error) => {
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

            // if let Err(error) = job_processor.submit_job(job).await {
            //     error!(?error, "failed while submitting to job processor");
            // }
            if let Some(n) = log.block_number {
                last_seen_block = BlockNumberOrTag::Number(n);
            }
        }

        sleep(Duration::from_millis(retry * 10)).await;
        warn!(?retry, ?last_seen_block, "websocket reconnecting");
        retry += 1;
    }
}
