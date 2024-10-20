// //! Collects requests into batches and submits them to the coprocessor node at some regular
// //! cadence.
// // use crate::db::{
// //     tables::{ClobStateTable, GlobalIndexTable, RequestTable},
// //     NEXT_BATCH_GLOBAL_INDEX_KEY, PROCESSED_GLOBAL_INDEX_KEY,
// // };
// use abi::{abi_encode_offchain_job_request, JobParams};
// use alloy::{primitives::utils::keccak256, signers::Signer};
// use simple_programs::SIMPLE_ID;
// use eyre::OptionExt;
// use proto::{coprocessor_node_client::CoprocessorNodeClient, SubmitJobRequest};
// use reth_db::transaction::{DbTx, DbTxMut};
// use reth_db_api::Database;
// use risc0_zkvm::sha::Digest;
// use std::sync::Arc;
// use tokio::time::{sleep, Duration};
// use tonic::transport::Channel;
// use tracing::{debug, info, instrument, warn};

// use crate::K256LocalSigner;

// const MAX_CYCLES: u64 = 32 * 1000 * 1000;
// /// First global index that a request will get.
// const INIT_INDEX: u64 = 1;

// // async fn ensure_initialized() -> eyre::Result<()>
// // {
//     // loop {
//     //     let has_processed =
//     //         db.view(|tx| tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY))??.is_some();
//     //     if has_processed {
//     //         let has_next_batch =
//     //             db.view(|tx| tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY))??.is_some();
//     //         if !has_next_batch {
//     //             db.update(|tx| {
//     //                 tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, INIT_INDEX)
//     //             })??;
//     //         }

//     //         break;
//     //     } else {
//     //         debug!("waiting for the first request to be processed before starting batcher");
//     //         sleep(Duration::from_millis(1_0000)).await;
//     //         continue;
//     //     }
//     // }

// //     Ok(())
// // }

// // pub async fn submit_batch(
// //     sleep_duration: Duration,
// //     signer: K256LocalSigner,
// //     cn_grpc_url: String,
// //     simple_consumer_addr: [u8; 20],
// // ) -> eyre::Result<()> {
// //     let program_id = Digest::from(SIMPLE_ID).as_bytes().to_vec();
// //     let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
// //     let mut interval = tokio::time::interval(sleep_duration);

// //     let mut job_nonce = 0;

// //     let job_params = JobParams {
// //         nonce: job_nonce,
// //         max_cycles: MAX_CYCLES,
// //         onchain_input: &[],
// //         offchain_input_hash: offchain_input_hash.into(),
// //         state_hash: previous_state_hash.into(),
// //         program_id: &program_id,
// //         consumer_address: clob_consumer_addr,
// //     };
// //     let request = abi_encode_offchain_job_request(job_params);
// //     let signature = signer.sign_message(&request).await?.as_bytes().to_vec();
// //     let job_request = SubmitJobRequest {
// //         request,
// //         signature,
// //         offchain_input: requests_borsh,
// //         state: todo!("Get this state passed in"),
// //     };

// //     submit_job_with_backoff(&mut coprocessor_node, job_request).await?;
// //     Ok(())

// // }

// // /// Run the CLOB execution engine
// // #[instrument(skip_all)]
// // pub async fn run_batcher<D>(
// //     db: Arc<D>,
// //     sleep_duration: Duration,
// //     signer: K256LocalSigner,
// //     cn_grpc_url: String,
// //     clob_consumer_addr: [u8; 20],
// // ) -> eyre::Result<()>
// // where
// //     D: Database + 'static,
// // {
// //     // Wait for the system to have at least one processed request from the CLOB server
// //     // ensure_initialized(Arc::clone(&db)).await?;
// //     let program_id = Digest::from(SIMPLE_ID).as_bytes().to_vec();

// //     let mut coprocessor_node = CoprocessorNodeClient::connect(cn_grpc_url).await.unwrap();
// //     let mut interval = tokio::time::interval(sleep_duration);

// //     let mut job_nonce = 0;
// //     loop {
// //         interval.tick().await;

// //         let start_index = db
// //             .view(|tx| tx.get::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY))??
// //             .ok_or_eyre("start_index")?;
// //         let end_index = db
// //             .view(|tx| tx.get::<GlobalIndexTable>(PROCESSED_GLOBAL_INDEX_KEY))??
// //             .ok_or_eyre("end_index")?;

// //         if start_index > end_index {
// //             debug!(start_index, end_index, "skipping batch creation");
// //             continue;
// //         }

// //         info!("creating batch {start_index}..={end_index}");

// //         let prev_state_index = start_index - 1;
// //         let start_state =
// //             db.view(|tx| tx.get::<ClobStateTable>(prev_state_index))??.ok_or_eyre("start_state")?.0;
// //         tokio::task::yield_now().await;

// //         let mut requests = Vec::with_capacity((end_index - start_index + 1) as usize);
// //         for i in start_index..=end_index {
// //             let r = db
// //                 .view(|tx| tx.get::<RequestTable>(i))??
// //                 .ok_or_eyre(format!("batcher: request {i}"))?
// //                 .0;
// //             requests.push(r);
// //             tokio::task::yield_now().await;
// //         }

// //         let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");
// //         let offchain_input_hash = keccak256(&requests_borsh);
// //         let state_borsh = borsh::to_vec(&start_state).expect("borsh works. qed.");
// //         let previous_state_hash = keccak256(&state_borsh);

// //         let job_params = JobParams {
// //             nonce: job_nonce,
// //             max_cycles: MAX_CYCLES,
// //             onchain_input: &[],
// //             offchain_input_hash: offchain_input_hash.into(),
// //             state_hash: previous_state_hash.into(),
// //             program_id: &program_id,
// //             consumer_address: clob_consumer_addr,
// //         };
// //         let request = abi_encode_offchain_job_request(job_params);
// //         let signature = signer.sign_message(&request).await?.as_bytes().to_vec();
// //         let job_request = SubmitJobRequest {
// //             request,
// //             signature,
// //             offchain_input: requests_borsh,
// //             state: state_borsh,
// //         };

// //         submit_job_with_backoff(&mut coprocessor_node, job_request).await?;

// //         let next_batch_idx = end_index + 1;
// //         db.update(|tx| tx.put::<GlobalIndexTable>(NEXT_BATCH_GLOBAL_INDEX_KEY, next_batch_idx))??;

// //         // TODO: https://github.com/InfinityVM/InfinityVM/issues/295
// //         // read highest job nonce from contract
// //         job_nonce += 1;
// //     }
// // }

// // TODO: implement simpler version
// async fn submit_job_with_backoff(
//     client: &mut CoprocessorNodeClient<Channel>,
//     request: SubmitJobRequest,
// ) -> eyre::Result<()> {
//     const RETRIES: usize = 3;
//     const MULTIPLE: u64 = 8;

//     let mut backoff = 1;

//     for _ in 0..RETRIES {
//         match client.submit_job(request.clone()).await {
//             Ok(_) => return Ok(()),
//             Err(error) => warn!(backoff, ?error, "failed to submit batch to coprocessor"),
//         }

//         sleep(Duration::from_secs(backoff)).await;
//         backoff *= MULTIPLE;
//     }

//     eyre::bail!("failed to submit batch to coprocessor after {RETRIES} retries")
// }
