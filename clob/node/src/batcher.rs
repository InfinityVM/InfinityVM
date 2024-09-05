//! Collects requests into batches and submits them to the coprocessor node at some regular
//! cadence.
use crate::db::{
    tables::{ClobStateTable, GlobalIndexTable, RequestTable},
    NEXT_BATCH_GLOBAL_INDEX_KEY, PROCESSED_GLOBAL_INDEX_KEY,
};
use abi::{abi_encode_offchain_job_request, JobParams};

use alloy::{primitives::utils::keccak256, signers::Signer, sol_types::SolType};

use clob_core::api::ClobProgramInput;
use clob_programs::CLOB_ID;
use proto::{coprocessor_node_client::CoprocessorNodeClient, SubmitStatefulJobRequest};
use reth_db::transaction::{DbTx, DbTxMut};
use reth_db_api::Database;
use risc0_zkvm::sha::Digest;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, instrument};

use crate::K256LocalSigner;

const MAX_CYCLES: u64 = 32 * 1000 * 1000;
/// First global index that a request will get.
const INIT_INDEX: u64 = 1;

async fn ensure_initialized<D>(db: Arc<D>)
where
    D: Database + 'static,
{
    loop {
        let has_processed = db
            .view(|tx| {
                tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY).expect("todo").is_some()
            })
            .unwrap();
        if has_processed {
            let has_next_batch = db
                .view(|tx| {
                    tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY).expect("todo").is_some()
                })
                .unwrap();
            if !has_next_batch {
                db.update(|tx| {
                    tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, INIT_INDEX)
                        .expect("todo")
                })
                .unwrap();
            }

            break;
        } else {
            info!("waiting for a request to be processed before starting batcher");
            sleep(Duration::from_millis(1_0000)).await;
            continue;
        }
    }
}

/// Run the CLOB execution engine
#[instrument(skip_all)]
pub async fn run_batcher<D>(
    db: Arc<D>,
    sleep_duration: Duration,
    signer: K256LocalSigner,
    cn_grpc_url: String,
    clob_consumer_addr: [u8; 20],
) -> eyre::Result<()>
where
    D: Database + 'static,
{
    // Wait for the system to have at least one processed request from the CLOB server
    ensure_initialized(Arc::clone(&db)).await;
    let program_id = Digest::from(CLOB_ID).as_bytes().to_vec();

    let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
    let mut interval = tokio::time::interval(sleep_duration);

    let mut job_nonce = 0;
    loop {
        interval.tick().await;

        let start_index = db
            .view(|tx| {
                tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY).expect("todo").unwrap()
            })
            .unwrap();
        let end_index = db
            .view(|tx| {
                tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY).expect("todo").unwrap()
            })
            .unwrap();

        if start_index > end_index {
            tracing::info!(start_index, end_index, "skipping batch creation");
            continue;
        }

        tracing::info!("creating batch {start_index}..={end_index}");

        let prev_state_index = start_index - 1;
        let start_state = db
            .view(|tx| tx.get::<ClobStateTable>(prev_state_index).expect("todo").unwrap().0)
            .unwrap();
        tokio::task::yield_now().await;

        let mut requests = vec![];
        for i in start_index..=end_index {
            let r = db.view(|tx| tx.get::<RequestTable>(i).expect("todo").unwrap().0).unwrap();
            requests.push(r);
            tokio::task::yield_now().await;
        }

        let requests_borsh = borsh::to_vec(&requests).expect("valid borsh");
        let program_state_borsh = borsh::to_vec(&start_state).expect("valid borsh");
        let prev_state_hash = keccak256(&program_state_borsh);

        let program_input = ClobProgramInput { prev_state_hash, orders: requests_borsh.into() };
        let program_input_encoded = ClobProgramInput::abi_encode(&program_input);

        let job_params = JobParams {
            nonce: job_nonce,
            max_cycles: MAX_CYCLES,
            program_input: &program_input_encoded,
            program_id: &program_id,
            consumer_address: clob_consumer_addr,
        };
        let request = abi_encode_offchain_job_request(job_params);
        let signature = signer.sign_message(&request).await.unwrap().as_bytes().to_vec();
        let job_request =
            SubmitStatefulJobRequest { request, signature, program_state: program_state_borsh };

        let _submit_stateful_job_response =
            coprocessor_node.submit_stateful_job(job_request).await.unwrap().into_inner();

        let next_batch_idx = end_index + 1;
        db.update(|tx| {
            tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, next_batch_idx).unwrap()
        })
        .unwrap();

        // TODO: read highest job nonce from contract
        job_nonce += 1;
    }
}
