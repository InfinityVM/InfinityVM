//! Test the coprocessor node by submitting programs, submitting jobs with onchain and offchain
//! inputs, and getting the result for both.

use alloy::{
    primitives::{hex, Address, FixedBytes},
    providers::ProviderBuilder,
    signers::local::LocalSigner,
    sol_types::SolValue,
};
use clob_programs::CLOB_ELF;
use contracts::mock_consumer::MockConsumer;
use ivm_abi::StatefulAppOnchainInput;
use ivm_proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, GetResultResponse,
    RelayStrategy, SubmitJobRequest, SubmitProgramRequest, VmType,
};
use ivm_test_utils::create_and_sign_offchain_request;
use k256::ecdsa::SigningKey;
use kairos_trie::{stored::memory_db::MemoryDb, TrieRoot};
use matching_game_core::{
    api::{Request, SubmitNumberRequest},
    get_merkle_root_bytes, next_state,
};
use matching_game_programs::{get_matching_game_program_id, MATCHING_GAME_ELF};
use matching_game_server::contracts::matching_game_consumer::MatchingGameConsumer;
use mock_consumer::MOCK_CONSUMER_MAX_CYCLES;
use mock_consumer_programs::{get_mock_consumer_program_id, MOCK_CONSUMER_ELF};
use std::rc::Rc;
use tracing::{error, info};
 use clob_programs::get_clob_program_id;

const COPROCESSOR_IP: &str = "";
const COPROCESSOR_GRPC_PORT: u16 = 50420;
const COPROCESSOR_HTTP_PORT: u16 = 8080;
const MOCK_CONSUMER_ADDR: &str = "0x124363b6D0866118A8b6899F2674856618E0Ea4c";
const MATCHING_GAME_CONSUMER_ADDR: &str = "0x5793a71D3eF074f71dCC21216Dbfd5C0e780132c";
const OFFCHAIN_SIGNER_PRIVATE_KEY: &str =
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6";
const ANVIL_IP: &str = "";
const ANVIL_PORT: u16 = 8545;

type K256LocalSigner = LocalSigner<SigningKey>;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let client_coproc_grpc = format!("http://{COPROCESSOR_IP}:{COPROCESSOR_GRPC_PORT}");
    let mut coproc_client =
        CoprocessorNodeClient::connect(client_coproc_grpc.clone()).await.unwrap();

    info!("Trying to submit CLOB ELF to coprocessor node");
    let submit_program_request =
        SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Sp1.into(), program_id: get_clob_program_id().to_vec() };
    // We handle error here because we don't want to panic if the ELF was already submitted
    match coproc_client.submit_program(submit_program_request).await {
        Ok(submit_program_response) => {
            info!("Submitted CLOB ELF to coprocessor node: {:?}", submit_program_response);
        }
        Err(e) => {
            error!("Error while submitting CLOB ELF to coprocessor node: {:?}", e);
        }
    }

    info!("Trying to submit MockConsumer ELF to coprocessor node");
    let submit_program_request = SubmitProgramRequest {
        program_elf: MOCK_CONSUMER_ELF.to_vec(),
        vm_type: VmType::Sp1.into(),
        program_id: get_mock_consumer_program_id().to_vec(),
    };
    // We handle error here because we don't want to panic if the ELF was already submitted
    match coproc_client.submit_program(submit_program_request).await {
        Ok(submit_program_response) => {
            info!("Submitted MockConsumer ELF to coprocessor node: {:?}", submit_program_response);
        }
        Err(e) => {
            error!("Error while submitting MockConsumer ELF to coprocessor node: {:?}", e);
        }
    }

    info!("Trying to submit MatchingGameConsumer ELF to coprocessor node");
    let submit_program_request = SubmitProgramRequest {
        program_elf: MATCHING_GAME_ELF.to_vec(),
        vm_type: VmType::Sp1.into(),
        program_id: get_matching_game_program_id().to_vec(),
    };
    // We handle error here because we don't want to panic if the ELF was already submitted
    match coproc_client.submit_program(submit_program_request).await {
        Ok(submit_program_response) => {
            info!(
                "Submitted MatchingGameConsumer ELF to coprocessor node: {:?}",
                submit_program_response
            );
        }
        Err(e) => {
            error!("Error while submitting MatchingGameConsumer ELF to coprocessor node: {:?}", e);
        }
    }

    let provider = ProviderBuilder::new().with_recommended_fillers().on_http(
        url::Url::parse(format!("http://{ANVIL_IP}:{ANVIL_PORT}").as_str()).expect("Valid URL"),
    );
    let decoded = hex::decode(OFFCHAIN_SIGNER_PRIVATE_KEY).unwrap();
    let offchain_signer = K256LocalSigner::from_slice(&decoded).unwrap();

    // Test mock consumer job submission flow

    // Get nonce from the mock consumer contract
    let mock_consumer_addr =
        Address::parse_checksummed(MOCK_CONSUMER_ADDR, None).expect("Valid address");
    let mock_consumer_contract = MockConsumer::new(mock_consumer_addr, &provider);
    let nonce = mock_consumer_contract.getNextNonce().call().await.unwrap()._0;
   
    let (encoded_job_request, signature) = create_and_sign_offchain_request(
        nonce,
        MOCK_CONSUMER_MAX_CYCLES,
        mock_consumer_addr,
        Address::abi_encode(&mock_consumer_addr).as_slice(),
        &get_mock_consumer_program_id()[..],
        offchain_signer.clone(),
        &[],
    )
    .await;

    // Submit a job to the coprocessor node
    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        relay_strategy: RelayStrategy::Unordered as i32,
    };

    let submit_job_response = coproc_client.submit_job(submit_job_request).await.unwrap();
    info!("Submitted job for mock consumer: {:?}", submit_job_response);

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    // Get the job result from the coprocessor node
    let job_id = submit_job_response.into_inner().job_id;
    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let get_result_response = coproc_client.get_result(get_result_request).await.unwrap();
    info!("Result for mock consumer: {:?}", get_result_response);

    // Call get_result on HTTP gateway as well to make sure the gateway works correctly
    let http_gateway_url = format!("http://{COPROCESSOR_IP}:{COPROCESSOR_HTTP_PORT}");
    let client = reqwest::Client::new();

    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let response = client
        .post(format!("{}/v1/coprocessor_node/get_result", http_gateway_url))
        .json(&get_result_request)
        .send()
        .await
        .expect("Failed to send request");

    let get_result_response: GetResultResponse =
        response.json().await.expect("Failed to parse JSON response");
    info!("HTTP Gateway Result for mock consumer: {:?}", get_result_response);

    // Wait for the job result to be relayed to anvil
    let mut inputs;
    let fixed_size_job_id: [u8; 32] = job_id.as_slice().try_into().expect("Fixed size array");
    let get_inputs_call =
        mock_consumer_contract.getOnchainInputForJob(FixedBytes(fixed_size_job_id));
    loop {
        match get_inputs_call.call().await {
            Ok(MockConsumer::getOnchainInputForJobReturn { _0: result }) => {
                inputs = result.to_vec();
                // If the inputs are all zero, then the job is not yet relayed
                if result.into_iter().all(|x| x == 0) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                } else {
                    // result successfully submitted onchain
                    break;
                }
            }
            Err(e) => eprintln!("Error calling getOnchainInputForJob: {:?}", e),
        }
    }

    info!("Job result for mock consumer relayed to anvil with inputs: {:x?}", inputs);

    // Test matching game job submission flow

    // Get nonce from the matching game consumer contract
    let matching_game_consumer_addr =
        Address::parse_checksummed(MATCHING_GAME_CONSUMER_ADDR, None).expect("Valid address");
    let matching_game_consumer_contract =
        MatchingGameConsumer::new(matching_game_consumer_addr, &provider);
    let nonce = matching_game_consumer_contract.getNextNonce().call().await.unwrap()._0;

    let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
    let pre_txn_merkle_root = TrieRoot::Empty;

    // Arbitrary addresses for alice and bob
    let alice = **mock_consumer_addr;
    let bob = **matching_game_consumer_addr;
    let requests = vec![
        Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
        Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
    ];
    let requests_bytes = bincode::serialize(&requests).unwrap();
    let (_, snapshot, _) = next_state(trie_db.clone(), pre_txn_merkle_root, &requests);
    let snapshot_bytes = bincode::serialize(&snapshot).unwrap();

    let mut combined_offchain_input = Vec::new();
    combined_offchain_input.extend_from_slice(&(requests_bytes.len() as u32).to_le_bytes());
    combined_offchain_input.extend_from_slice(&requests_bytes);
    combined_offchain_input.extend_from_slice(&snapshot_bytes);

    let onchain_input = StatefulAppOnchainInput {
        input_state_root: get_merkle_root_bytes(pre_txn_merkle_root).into(),
        onchain_input: [0].into(),
    };
    let onchain_input_abi_encoded = StatefulAppOnchainInput::abi_encode(&onchain_input);

    let matching_game_program_id = get_matching_game_program_id();

    let (encoded_job_request, signature) = create_and_sign_offchain_request(
        nonce,
        MOCK_CONSUMER_MAX_CYCLES,
        matching_game_consumer_addr,
        onchain_input_abi_encoded.as_slice(),
        &matching_game_program_id,
        offchain_signer.clone(),
        &combined_offchain_input,
    )
    .await;

    // Submit a job to the coprocessor node
    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: combined_offchain_input,
        relay_strategy: RelayStrategy::Unordered as i32,
    };

    let submit_job_response = coproc_client.submit_job(submit_job_request).await.unwrap();
    info!("Submitted job for matching game: {:?}", submit_job_response);

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    // Get the job result from the coprocessor node
    let job_id = submit_job_response.into_inner().job_id;
    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let get_result_response = coproc_client.get_result(get_result_request).await.unwrap();
    info!("Result for matching game: {:?}", get_result_response);

    // Call get_result on HTTP gateway as well to make sure the gateway works correctly
    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let response = client
        .post(format!("{}/v1/coprocessor_node/get_result", http_gateway_url))
        .json(&get_result_request)
        .send()
        .await
        .expect("Failed to send request");

    let get_result_response: GetResultResponse =
        response.json().await.expect("Failed to parse JSON response");
    info!("HTTP Gateway Result for matching game: {:?}", get_result_response);

    // Wait for the job result to be relayed to anvil
    let mut inputs;
    let fixed_size_job_id: [u8; 32] = job_id.as_slice().try_into().expect("Fixed size array");
    let get_inputs_call =
        matching_game_consumer_contract.getOnchainInputForJob(FixedBytes(fixed_size_job_id));
    loop {
        match get_inputs_call.call().await {
            Ok(MatchingGameConsumer::getOnchainInputForJobReturn { _0: result }) => {
                inputs = result.to_vec();
                // If the inputs are all zero, then the job is not yet relayed
                if result.into_iter().all(|x| x == 0) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                } else {
                    // result successfully submitted onchain
                    break;
                }
            }
            Err(e) => eprintln!("Error calling getOnchainInputForJob: {:?}", e),
        }
    }

    info!("Job result for matching game relayed to anvil with inputs: {:x?}", inputs);
}
