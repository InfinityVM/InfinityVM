//! Locally spin up the coprocessor node, anvil, and the clob.

use alloy::{
    network::EthereumWallet,
    primitives::{hex, Address, FixedBytes},
    providers::ProviderBuilder,
    signers::{local::LocalSigner, Signer},
    sol_types::SolValue,
};
use clob_node::{
    CLOB_BATCHER_DURATION_MS, CLOB_CN_GRPC_ADDR, CLOB_CONSUMER_ADDR, CLOB_DB_DIR, CLOB_ETH_WS_ADDR,
    CLOB_JOB_SYNC_START, CLOB_LISTEN_ADDR, CLOB_OPERATOR_KEY,
};
use clob_programs::CLOB_ELF;
use clob_test_utils::{anvil_with_clob_consumer, mint_and_approve};
use contracts::{mock_consumer::MockConsumer, DeployInfo, DEFAULT_DEPLOY_INFO};
use k256::ecdsa::SigningKey;
use mock_consumer::anvil_with_mock_consumer;
use mock_consumer_methods::{MOCK_CONSUMER_GUEST_ELF, MOCK_CONSUMER_GUEST_ID};
use proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, SubmitJobRequest,
    SubmitProgramRequest, VmType,
};
use std::{fs::File, process::Command};
use test_utils::{
    anvil_with_job_manager, create_and_sign_offchain_request, sleep_until_bound, ProcKill,
    LOCALHOST,
};
use tokio::signal::unix::{signal, SignalKind};
use tracing::info;

const COPROCESSOR_IP: &str = "34.82.138.182";
const COPROCESSOR_GRPC_PORT: u16 = 50420;
const MOCK_CONSUMER_ADDR: &str = "0x0165878A594ca255338adfa4d48449f69242Eb8F";
const OFFCHAIN_SIGNER_PRIVATE_KEY: &str =
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6";
const ANVIL_IP: &str = "35.230.81.89";
const ANVIL_PORT: u16 = 8545;

type K256LocalSigner = LocalSigner<SigningKey>;

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let client_coproc_grpc = format!("http://{COPROCESSOR_IP}:{COPROCESSOR_GRPC_PORT}");

    let mut coproc_client =
        CoprocessorNodeClient::connect(client_coproc_grpc.clone()).await.unwrap();
    // info!("Submitting CLOB ELF to coprocessor node");
    // let submit_program_request =
    //     SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into() };
    // coproc_client.submit_program(submit_program_request).await.unwrap();

    // // We submit the MockConsumer ELF to the coprocessor node for load testing.
    // info!("Submitting MockConsumer ELF to coprocessor node");
    // let submit_program_request = SubmitProgramRequest {
    //     program_elf: MOCK_CONSUMER_GUEST_ELF.to_vec(),
    //     vm_type: VmType::Risc0.into(),
    // };
    // coproc_client.submit_program(submit_program_request).await.unwrap();

    let nonce = 3;
    let consumer_addr =
        Address::parse_checksummed(MOCK_CONSUMER_ADDR, None).expect("Valid address");
    let decoded = hex::decode(OFFCHAIN_SIGNER_PRIVATE_KEY).unwrap();
    let offchain_signer = K256LocalSigner::from_slice(&decoded).unwrap();

    let (encoded_job_request, signature) = create_and_sign_offchain_request(
        nonce,
        1000000,
        consumer_addr,
        Address::abi_encode(&consumer_addr).as_slice(),
        &MOCK_CONSUMER_GUEST_ID.iter().flat_map(|&x| x.to_le_bytes()).collect::<Vec<u8>>(),
        offchain_signer.clone(),
        &[],
        &[],
    )
    .await;

    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };

    let submit_job_response = coproc_client.submit_job(submit_job_request).await.unwrap();
    info!("Submitted job: {:?}", submit_job_response);

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    let job_id = submit_job_response.into_inner().job_id;
    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let get_result_response = coproc_client.get_result(get_result_request).await.unwrap();
    info!("Result: {:?}", get_result_response);

    let consumer_provider = ProviderBuilder::new().with_recommended_fillers().on_http(
        url::Url::parse(format!("http://{ANVIL_IP}:{ANVIL_PORT}").as_str()).expect("Valid URL"),
    );

    let consumer_contract = MockConsumer::new(consumer_addr, &consumer_provider);
    let mut inputs = vec![0u8; 32];
    // If the inputs are all zero, then the job is not yet relayed
    while inputs.iter().all(|&x| x == 0) {
        let fixed_size_job_id: [u8; 32] = job_id.as_slice().try_into().expect("Fixed size array");
        let get_inputs_call =
            consumer_contract.getOnchainInputForJob(FixedBytes(fixed_size_job_id));
        match get_inputs_call.call().await {
            Ok(MockConsumer::getOnchainInputForJobReturn { _0: result }) => {
                inputs = result.to_vec();
                if inputs.iter().all(|&x| x == 0) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
            Err(e) => eprintln!("Error calling getOnchainInputForJob: {:?}", e),
        }
    }

    info!("Job result relayed to anvil with inputs: {:?}", inputs);
}
