//! Test the coprocessor node by submitting programs, submitting a job, and getting the result.

use alloy::{
    primitives::{hex, Address, FixedBytes},
    providers::ProviderBuilder,
    signers::local::LocalSigner,
    sol_types::SolValue,
};
use clob_programs::CLOB_ELF;
use contracts::mock_consumer::MockConsumer;
use k256::ecdsa::SigningKey;
use mock_consumer::MOCK_CONSUMER_MAX_CYCLES;
use mock_consumer_methods::{MOCK_CONSUMER_GUEST_ELF, MOCK_CONSUMER_GUEST_ID};
use proto::{
    coprocessor_node_client::CoprocessorNodeClient, GetResultRequest, GetResultResponse,
    SubmitJobRequest, SubmitProgramRequest, VmType,
};
use test_utils::create_and_sign_offchain_request;
use tracing::{error, info};

const COPROCESSOR_IP: &str = "34.82.138.182";
const COPROCESSOR_GRPC_PORT: u16 = 50420;
const COPROCESSOR_HTTP_PORT: u16 = 8080;
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

    info!("Trying to submit CLOB ELF to coprocessor node");
    let submit_program_request =
        SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into() };
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
        program_elf: MOCK_CONSUMER_GUEST_ELF.to_vec(),
        vm_type: VmType::Risc0.into(),
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

    // Get nonce from the consumer contract
    let provider = ProviderBuilder::new().with_recommended_fillers().on_http(
        url::Url::parse(format!("http://{ANVIL_IP}:{ANVIL_PORT}").as_str()).expect("Valid URL"),
    );
    let consumer_addr =
        Address::parse_checksummed(MOCK_CONSUMER_ADDR, None).expect("Valid address");
    let consumer_contract = MockConsumer::new(consumer_addr, &provider);
    let nonce = consumer_contract.getNextNonce().call().await.unwrap()._0;

    let decoded = hex::decode(OFFCHAIN_SIGNER_PRIVATE_KEY).unwrap();
    let offchain_signer = K256LocalSigner::from_slice(&decoded).unwrap();

    let (encoded_job_request, signature) = create_and_sign_offchain_request(
        nonce,
        MOCK_CONSUMER_MAX_CYCLES,
        consumer_addr,
        Address::abi_encode(&consumer_addr).as_slice(),
        &MOCK_CONSUMER_GUEST_ID.iter().flat_map(|&x| x.to_le_bytes()).collect::<Vec<u8>>(),
        offchain_signer.clone(),
        &[],
        &[],
    )
    .await;

    // Submit a job to the coprocessor node
    let submit_job_request = SubmitJobRequest {
        request: encoded_job_request,
        signature,
        offchain_input: Vec::new(),
        state: Vec::new(),
    };

    let submit_job_response = coproc_client.submit_job(submit_job_request).await.unwrap();
    info!("Submitted job: {:?}", submit_job_response);

    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    // Get the job result from the coprocessor node
    let job_id = submit_job_response.into_inner().job_id;
    let get_result_request = GetResultRequest { job_id: job_id.clone() };
    let get_result_response = coproc_client.get_result(get_result_request).await.unwrap();
    info!("Result: {:?}", get_result_response);

    // Call get_result on HTTP gateway as well to make sure the gateway works correctly
    let http_gateway_url = format!("http://{COPROCESSOR_IP}:{COPROCESSOR_HTTP_PORT}");
    let client = reqwest::Client::new();

    let get_result_request = GetResultRequest { job_id: job_id.to_vec() };
    let response = client
        .post(format!("{}/v1/coprocessor_node/get_result", http_gateway_url))
        .json(&get_result_request)
        .send()
        .await
        .expect("Failed to send request");

    let get_result_response: GetResultResponse =
        response.json().await.expect("Failed to parse JSON response");
    info!("HTTP Gateway Result: {:?}", get_result_response);

    // Wait for the job result to be relayed to anvil
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

    info!("Job result relayed to anvil with inputs: {:x?}", inputs);
}
