use abi::abi_encode_offchain_job_request;
use alloy::{
    primitives::{hex, keccak256, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use db::tables::{get_job_id, Job, RequestType};
use dotenv::dotenv;
use k256::ecdsa::SigningKey;
use proto::{JobStatus, JobStatusType};
use std::env;
use zkvm_executor::service::{
    abi_encode_offchain_result_with_metadata, abi_encode_result_with_metadata,
};
use test_utils::get_signers;

type K256LocalSigner = LocalSigner<SigningKey>;

const MAX_CYCLES: u64 = 1_000_000;
const PROGRAM_ID: &[u8] = b"programID";
const NONCE: u64 = 1;
const CONSUMER_ADDR: &str = "0xDB8cFf278adCCF9E9b5da745B44E754fC4EE3C76";
const LOCAL_SETUP_CONSUMER_ADDR: &str = "0xbdEd0D2bf404bdcBa897a74E6657f1f12e5C6fb6";
const LOCAL_SETUP_PROGRAM_ID: &[u8] = &[38, 97, 129, 246, 1, 9, 102, 56, 121, 187, 170, 57, 163, 102, 31, 208, 122, 142, 221, 113, 246, 162, 114, 4, 239, 24, 213, 94, 45, 195, 127, 233];

/// Script to generate ABI-encoded responses + signatures for the coprocessor contract tests
#[derive(Debug)]
pub struct RequestAndResultSigner;

impl RequestAndResultSigner {
    /// Sign a result for a job requested onchain
    pub async fn sign_onchain_result() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;

        // Encode the result with metadata
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_result = abi_encode_result_with_metadata(
            get_job_id(NONCE, Address::parse_checksummed(CONSUMER_ADDR, None).unwrap()),
            keccak256(Address::abi_encode(&zero_addr)),
            MAX_CYCLES,
            PROGRAM_ID,
            &raw_output,
        );

        // Sign the message
        let private_key_hex = env::var("COPROCESSOR_OPERATOR_PRIVATE_KEY")
            .expect("COPROCESSOR_OPERATOR_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_result).await.unwrap();

        println!("Encoded onchain result: {}", hex::encode(&encoded_result));
        println!("Signature for encoded onchain result: {}", hex::encode(signature.as_bytes()));
    }

    /// Sign a result for a job requested offchain
    pub async fn sign_offchain_result() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;

        // Encode the offchain result with metadata
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_result = abi_encode_offchain_result_with_metadata(
            get_job_id(NONCE, Address::parse_checksummed(CONSUMER_ADDR, None).unwrap()),
            keccak256(Address::abi_encode(&zero_addr)),
            keccak256(vec![]),
            keccak256(vec![]),
            MAX_CYCLES,
            PROGRAM_ID,
            &raw_output,
        );

        // Sign the message
        let private_key_hex = env::var("COPROCESSOR_OPERATOR_PRIVATE_KEY")
            .expect("COPROCESSOR_OPERATOR_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_result).await.unwrap();

        println!("Encoded offchain result: {}", hex::encode(&encoded_result));
        println!("Signature for encoded offchain result: {}", hex::encode(signature.as_bytes()));
    }

    /// Sign an offchain job request
    pub async fn sign_job_request() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;
        let consumer_addr: Address = Address::parse_checksummed(LOCAL_SETUP_CONSUMER_ADDR, None).unwrap();

        let job = Job {
            id: get_job_id(NONCE, consumer_addr),
            nonce: NONCE,
            max_cycles: MAX_CYCLES,
            // Need to use abi_encode_packed because the contract address
            // should not be zero-padded
            consumer_address: Address::abi_encode_packed(&consumer_addr),
            program_id: LOCAL_SETUP_PROGRAM_ID.to_vec(),
            onchain_input: Address::abi_encode(&consumer_addr),
            offchain_input: vec![],
            state: vec![],
            request_type: RequestType::Offchain(vec![]),
            result_with_metadata: vec![],
            zkvm_operator_signature: vec![],
            status: JobStatus {
                status: JobStatusType::Pending as i32,
                failure_reason: None,
                retries: 0,
            },
        };
        let job_params = (&job).try_into().unwrap();
        let encoded_job_request = abi_encode_offchain_job_request(job_params);

        let signers = get_signers(6);
        let offchain_signer = signers[5].clone();

        // let private_key_hex = env::var("OFFCHAIN_SIGNER_PRIVATE_KEY")
        //     .expect("OFFCHAIN_SIGNER_PRIVATE_KEY not set in .env file");
        // let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        // let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = offchain_signer.sign_message(&encoded_job_request).await.unwrap();

        println!("Encoded job request: {:?}", encoded_job_request);
        println!("Signature for encoded job request: {:?}", signature.as_bytes());
    }
}

type AddressWithBalance = sol! {
    tuple(address,uint256)
};

fn abi_encode_address_with_balance(address: Address, balance: U256) -> Vec<u8> {
    AddressWithBalance::abi_encode(&(address, balance))
}
