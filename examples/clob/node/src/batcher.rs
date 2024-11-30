//! Collects requests into batches and submits them to the coprocessor node at some regular
//! cadence.
use crate::{
    db::{
        tables::{ClobStateTable, GlobalIndexTable, RequestTable},
        NEXT_BATCH_GLOBAL_INDEX_KEY, PROCESSED_GLOBAL_INDEX_KEY,
    },
    K256LocalSigner,
};
use alloy::{primitives::utils::keccak256, signers::Signer, sol_types::SolValue};
use clob_programs::CLOB_PROGRAM_ID;
use eyre::OptionExt;
use ivm_abi::{abi_encode_offchain_job_request, JobParams, StatefulAppOnchainInput};
use ivm_proto::{coprocessor_node_client::CoprocessorNodeClient, RelayStrategy, SubmitJobRequest};
use reth_db::transaction::{DbTx, DbTxMut};
use reth_db_api::Database;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tonic::transport::Channel;
use tracing::{debug, info, instrument, warn};

const MAX_CYCLES: u64 = 32 * 1000 * 1000;
/// First global index that a request will get.
const INIT_INDEX: u64 = 1;

async fn ensure_initialized<D>(db: Arc<D>) -> eyre::Result<()>
where
    D: Database + 'static,
{
    loop {
        let has_processed =
            db.view(|tx| tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY))??.is_some();
        if has_processed {
            let has_next_batch =
                db.view(|tx| tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY))??.is_some();
            if !has_next_batch {
                db.update(|tx| {
                    tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, INIT_INDEX)
                })??;
            }

            break;
        } else {
            debug!("waiting for the first request to be processed before starting batcher");
            sleep(Duration::from_millis(1_0000)).await;
            continue;
        }
    }

    Ok(())
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
    ensure_initialized(Arc::clone(&db)).await?;
    let program_id = CLOB_PROGRAM_ID;

    let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
    let mut interval = tokio::time::interval(sleep_duration);

    let mut job_nonce = 0;
    loop {
        interval.tick().await;

        let start_index = db
            .view(|tx| tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY))??
            .ok_or_eyre("start_index")?;
        let end_index = db
            .view(|tx| tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY))??
            .ok_or_eyre("end_index")?;

        if start_index > end_index {
            debug!(start_index, end_index, "skipping batch creation");
            continue;
        }

        info!("creating batch {start_index}..={end_index}");

        let prev_state_index = start_index - 1;
        let start_state =
            db.view(|tx| tx.get::<ClobStateTable>(prev_state_index))??.ok_or_eyre("start_state")?.0;
        tokio::task::yield_now().await;

        let mut requests = Vec::with_capacity((end_index - start_index + 1) as usize);
        for i in start_index..=end_index {
            let r = db
                .view(|tx| tx.get::<RequestTable>(i))??
                .ok_or_eyre(format!("batcher: request {i}"))?
                .0;
            requests.push(r);
            tokio::task::yield_now().await;
        }

        let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");
        let state_borsh = borsh::to_vec(&start_state).expect("borsh works. qed.");

        // Combine requests_borsh and state_borsh
        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&state_borsh);

        let offchain_input_hash = keccak256(&combined_offchain_input);
        let state_hash = keccak256(&state_borsh);

        let onchain_input =
            StatefulAppOnchainInput { input_state_root: state_hash, onchain_input: (&[]).into() };
        let onchain_input_abi_encoded = StatefulAppOnchainInput::abi_encode(&onchain_input);

        // TODO (Maanav): add state root to onchain input and add state merkle proofs to offchain
        // input
        // [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
        let job_params = JobParams {
            nonce: job_nonce,
            max_cycles: MAX_CYCLES,
            onchain_input: onchain_input_abi_encoded.as_slice(),
            offchain_input_hash: offchain_input_hash.into(),
            program_id: &program_id,
            consumer_address: clob_consumer_addr,
        };
        let request = abi_encode_offchain_job_request(job_params);
        let signature = signer.sign_message(&request).await?.as_bytes().to_vec();
        let job_request = SubmitJobRequest {
            request,
            signature,
            offchain_input: combined_offchain_input,
            relay_strategy: RelayStrategy::Ordered as i32,
        };

        submit_job_with_backoff(&mut coprocessor_node, job_request).await?;

        let next_batch_idx = end_index + 1;
        db.update(|tx| tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, next_batch_idx))??;

        // TODO: https://github.com/InfinityVM/InfinityVM/issues/295
        // read highest job nonce from contract
        job_nonce += 1;
    }
}

async fn submit_job_with_backoff(
    client: &mut CoprocessorNodeClient<Channel>,
    request: SubmitJobRequest,
) -> eyre::Result<()> {
    const RETRIES: usize = 3;
    const MULTIPLE: u64 = 8;

    let mut backoff = 1;

    for _ in 0..RETRIES {
        match client.submit_job(request.clone()).await {
            Ok(_) => return Ok(()),
            Err(error) => warn!(backoff, ?error, "failed to submit batch to coprocessor"),
        }

        sleep(Duration::from_secs(backoff)).await;
        backoff *= MULTIPLE;
    }

    eyre::bail!("failed to submit batch to coprocessor after {RETRIES} retries")
}
