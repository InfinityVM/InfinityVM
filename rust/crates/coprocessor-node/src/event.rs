//! Job request event listener.

use alloy::{
    eips::BlockNumberOrTag,
    network::{Ethereum, EthereumWallet, TxSigner},
    primitives::Address,
    providers::{ProviderBuilder, WsConnect},
    rpc::types::{Filter, TransactionReceipt},
    signers::Signature,
    sol_types::SolEvent,
    transports::http::reqwest,
};
use async_channel::Sender;
use contracts::job_manager::JobManager;
// use futures_util::StreamExt;
use proto::Job;
use tracing::{error, info};

/// Errors from the job request event listener
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("event subscription: {0}")]
    Subscription(#[from] alloy::transports::TransportError<alloy::transports::TransportErrorKind>),
}

pub async fn listen_job_request_events(
    http_rpc_url: String,
    job_manager: Address,
    exec_queue_sender: Sender<Job>,
    from_block: BlockNumberOrTag,
) -> Result<(), Error> {
    // let provider = ProviderBuilder::new().on_http(http_rpc_url.parse().expect("todo"));

    let ws = WsConnect::new(http_rpc_url);
    let provider = ProviderBuilder::new().on_ws(ws).await?;

    let contract = JobManager::new(job_manager, provider);

    let sub = contract.JobCreated_filter().from_block(from_block).subscribe().await?;

    let mut stream = sub.into_stream();
    for event in stream {

    }

    let sub = provider.subscribe_logs(&filter).await?;
    let mut stream = sub.into_stream();
    unimplemented!();
}
