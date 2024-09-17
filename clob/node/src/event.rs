//! Deposit event listener.

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::{ApiResponse, DepositRequest, Request};
use eyre::WrapErr;
use futures_util::StreamExt;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    time::{sleep, Duration},
};
use tracing::{debug, error, warn};

const FIVE_MINUTES_MILLIS: u64 = 300000;

/// Listen for deposit events and push a corresponding
/// `Deposit` request
pub async fn start_deposit_event_listener(
    ws_rpc_url: String,
    clob_consumer: Address,
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    from_block: BlockNumberOrTag,
) -> eyre::Result<()> {
    let mut last_seen_block = from_block;

    let mut provider_retry = 1;
    let provider = loop {
        let ws = WsConnect::new(ws_rpc_url.clone());
        let p = ProviderBuilder::new().on_ws(ws).await;
        match p {
            Ok(p) => break p,
            Err(_) => {
                let sleep_millis = provider_retry * 3;
                sleep(Duration::from_millis(sleep_millis)).await;
                if sleep_millis < FIVE_MINUTES_MILLIS {
                    provider_retry += 1;
                }
                debug!(?sleep_millis, "retrying creating ws connection");
                continue;
            }
        }
    };

    let contract = ClobConsumer::new(clob_consumer, &provider);
    let event_stream_retry = 1;
    loop {
        // We have this loop so we can recreate a subscription stream in case any issue is
        // encountered
        let sub = match contract.Deposit_filter().from_block(last_seen_block).subscribe().await {
            Ok(sub) => sub,
            Err(error) => {
                warn!(?error, "deposit event listener error");
                continue;
            }
        };
        let mut stream = sub.into_stream();

        while let Some(event) = stream.next().await {
            let (event, log) = match event {
                Ok((event, log)) => (event, log),
                Err(error) => {
                    error!(?error, "event listener");
                    break;
                }
            };

            let req = DepositRequest {
                address: **event.user,
                base_free: event.baseAmount.try_into()?,
                quote_free: event.quoteAmount.try_into()?,
            };
            let (tx, rx) = oneshot::channel::<ApiResponse>();
            engine_sender
                .send((Request::Deposit(req), tx))
                .await
                .wrap_err("engine receive unexpectedly dropped")?;
            let _resp = rx.await.wrap_err("engine oneshot sender unexpectedly dropped")?;

            if let Some(n) = log.block_number {
                last_seen_block = BlockNumberOrTag::Number(n);
            }
        }

        let sleep_millis = provider_retry * 10;
        sleep(Duration::from_millis(sleep_millis)).await;
        warn!(?event_stream_retry, ?last_seen_block, "retrying event stream creation");
        if sleep_millis < FIVE_MINUTES_MILLIS {
            provider_retry += 1;
        }
    }
}
