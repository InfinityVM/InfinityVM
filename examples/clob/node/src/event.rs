//! Deposit event listener.

use crate::db::{tables::BlockHeightTable, LAST_SEEN_HEIGHT_KEY};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{Provider, ProviderBuilder, WsConnect},
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::{ApiResponse, DepositRequest, Request};
use eyre::WrapErr;
use reth_db::transaction::{DbTx, DbTxMut};
use reth_db_api::Database;
use std::sync::Arc;
use tokio::{
    sync::{mpsc::Sender, oneshot},
    time::{sleep, Duration},
};
use tracing::{debug, warn};

const FIVE_MINUTES_MILLIS: u64 = 5 * 60 * 1000;

/// Listen for deposit events and push a corresponding
/// `Deposit` request
pub async fn start_deposit_event_listener<D>(
    db: Arc<D>,
    ws_rpc_url: String,
    clob_consumer: Address,
    engine_sender: Sender<(Request, oneshot::Sender<ApiResponse>)>,
    from_block: BlockNumberOrTag,
) -> eyre::Result<()>
where
    D: Database + 'static,
{
    let mut last_seen_block = from_block.as_number().unwrap_or_default();
    if let Some(last_saved_height) =
        db.view(|tx| tx.get::<BlockHeightTable>(LAST_SEEN_HEIGHT_KEY))??
    {
        if last_saved_height > last_seen_block {
            last_seen_block = last_saved_height;
        }
    }

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
            }
        }
    };

    let contract = ClobConsumer::new(clob_consumer, &provider);
    loop {
        let latest_block = provider.get_block_number().await?;

        while last_seen_block <= latest_block {
            let events = match contract
                .Deposit_filter()
                .from_block(last_seen_block)
                .to_block(last_seen_block)
                .query()
                .await
            {
                Ok(events) => events,
                Err(error) => {
                    warn!(?error, "deposit event listener error");
                    continue;
                }
            };

            for (event, _) in events {
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
            }

            last_seen_block += 1;
            db.update(|tx| tx.put::<BlockHeightTable>(LAST_SEEN_HEIGHT_KEY, last_seen_block))??;
        }
    }
}
