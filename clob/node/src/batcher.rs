//! Collects `BalanceChange`s into batches and submits then to the coprocessor node at some regular
//! cadence.
use crate::db::{
    tables::{ClobStateTable, GlobalIndexTable, RequestTable},
    NEXT_BATCH_GLOBAL_INDEX_KEY, PROCESSED_GLOBAL_INDEX_KEY,
};
use alloy::signers::Signer;
use clob_contracts::{abi_encode_offchain_job_request, JobParams};
use clob_programs::CLOB_ID;
use proto::SubmitJobRequest;
use reth_db::transaction::{DbTx, DbTxMut};
use reth_db_api::Database;
use risc0_zkvm::sha::Digest;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, instrument};

use crate::K256LocalSigner;

const MAX_CYCLES: u64 = 32 * 1000 * 1000;

async fn ensure_initialized<D>(db: Arc<D>)
where
    D: Database + 'static,
{
    loop {
        let tx = db.tx_mut().expect("todo");

        if tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY).expect("todo").is_some() {
            match tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY).expect("todo") {
                Some(_) => break,
                None => tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, 0).expect("todo"),
            }
        } else {
            info!("waiting for a request to be processed before starting batcher");
            sleep(Duration::from_millis(1_0000)).await;
            continue
        }

        let _ = tx.commit();
    }
}

/// Run the CLOB execution engine
#[instrument(skip_all)]
pub async fn run_batcher<D>(
    db: Arc<D>,
    sleep_duration: Duration,
    signer: K256LocalSigner,
    _cn_grpc_url: String,
    clob_consumer_addr: [u8; 20],
) where
    D: Database + 'static,
{
    // Wait for the system to have at least one processed request least one request
    ensure_initialized(Arc::clone(&db)).await;
    let program_id = Digest::from(CLOB_ID).as_bytes().to_vec();

    loop {
        sleep(sleep_duration).await;

        let tx = db.tx().expect("todo");
        let start_index =
            tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY).expect("todo").unwrap();
        let end_index =
            tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY).expect("todo").unwrap();

        if start_index >= end_index {
            info!("no new requests - skipping batch");
            continue;
        }
        let start_state = tx.get::<ClobStateTable>(start_index).expect("todo").unwrap().0;
        let requests: Vec<_> = (start_index..=end_index)
            .map(|index| tx.get::<RequestTable>(index).expect("todo").unwrap().0)
            .collect();
        let _ = tx.commit();

        let program_input = borsh::to_vec(&(requests, start_state)).expect("valid borsh");

        let job_params = JobParams {
            nonce: 0,
            max_cycles: MAX_CYCLES,
            program_input,
            program_id: program_id.clone(),
            consumer_address: clob_consumer_addr,
        };
        let request = abi_encode_offchain_job_request(job_params);
        let signature = signer.sign_message(&request).await.unwrap().as_bytes().to_vec();
        let _job_request = SubmitJobRequest { request, signature };

        let tx = db.tx_mut().expect("todo");
        tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, end_index + 1).expect("todo");
        let _ = tx.commit();
    }
}
