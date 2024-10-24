use abi::get_job_id;
use alloy::{
    primitives::{hex, keccak256, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use dotenv::dotenv;
use k256::ecdsa::SigningKey;
use std::env;
use test_utils::create_and_sign_offchain_request;
use zkvm_executor::service::{
    abi_encode_offchain_result_with_metadata, abi_encode_result_with_metadata,
};

type K256LocalSigner = LocalSigner<SigningKey>;

const MAX_CYCLES: u64 = 1_000_000;
const PROGRAM_ID: &[u8] = b"programID";
const NONCE: u64 = 1;
const CONSUMER_ADDR: &str = "0xBb2180ebd78ce97360503434eD37fcf4a1Df61c3";

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
        let consumer_addr: Address = Address::parse_checksummed(CONSUMER_ADDR, None).unwrap();

        let private_key_hex = env::var("OFFCHAIN_SIGNER_PRIVATE_KEY")
            .expect("OFFCHAIN_SIGNER_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();

        let (encoded_job_request, signature) = create_and_sign_offchain_request(
            NONCE,
            MAX_CYCLES,
            consumer_addr,
            Address::abi_encode(&zero_addr).as_slice(),
            PROGRAM_ID,
            signer,
            &[],
        )
        .await;

        println!("Encoded job request: {}", hex::encode(&encoded_job_request));
        println!("Signature for encoded job request: {}", hex::encode(signature));
    }
}

type AddressWithBalance = sol! {
    tuple(address,uint256)
};

fn abi_encode_address_with_balance(address: Address, balance: U256) -> Vec<u8> {
    AddressWithBalance::abi_encode(&(address, balance))
}
