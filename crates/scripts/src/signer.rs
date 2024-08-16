use alloy::{
    primitives::{hex, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
    sol,
    sol_types::{SolType, SolValue},
};
use coprocessor_node::job_processor::abi_encode_offchain_job_request;
use db::tables::get_job_id;
use dotenv::dotenv;
use k256::ecdsa::SigningKey;
use std::env;
use zkvm_executor::service::abi_encode_result_with_metadata;

type K256LocalSigner = LocalSigner<SigningKey>;

const MAX_CYCLES: u64 = 1_000_000;
const PROGRAM_ELF: &[u8] = b"elf";
const PROGRAM_ID: &[u8] = b"programID";
const VM_TYPE: i32 = 0;
const NONCE: u64 = 1;
const CONSUMER_ADDR: &str = "0xDB8cFf278adCCF9E9b5da745B44E754fC4EE3C76";

/// Script to generate ABI-encoded responses + signatures for the coprocessor contract tests
#[derive(Debug)]
pub struct RequestAndResultSigner;

impl RequestAndResultSigner {
    /// Sign a result for a job requested onchain
    pub async fn sign_onchain_result() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;

        let job_inputs = JobInputs {
            job_id: get_job_id(NONCE, Address::parse_checksummed(CONSUMER_ADDR, None).unwrap())
                .to_vec(),
            program_input: Address::abi_encode(&zero_addr),
            max_cycles: MAX_CYCLES,
            program_id: PROGRAM_ID.to_vec(),
            program_elf: PROGRAM_ELF.to_vec(),
            vm_type: VM_TYPE,
        };

        // Encode the result with metadata
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_result = abi_encode_result_with_metadata(&job_inputs, &raw_output).unwrap();

        // Sign the message
        let private_key_hex = env::var("COPROCESSOR_OPERATOR_PRIVATE_KEY")
            .expect("COPROCESSOR_OPERATOR_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_result).await.unwrap();

        println!("Encoded onchain result: {}", hex::encode(&encoded_result));
        println!("Signature for encoded onchain result: {}", hex::encode(signature.as_bytes()));
    }

    /// Sign an offchain job request
    pub async fn sign_job_request() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;
        let consumer_addr: Address = Address::parse_checksummed(CONSUMER_ADDR, None).unwrap();

        let job = Job {
            id: vec![],
            nonce: NONCE,
            max_cycles: MAX_CYCLES,
            // Need to use abi_encode_packed because the contract address
            // should not be zero-padded
            consumer_address: Address::abi_encode_packed(&consumer_addr),
            program_id: PROGRAM_ID.to_vec(),
            input: Address::abi_encode(&zero_addr),
            request_signature: vec![],
            result: vec![],
            zkvm_operator_signature: vec![],
            status: None,
        };
        let encoded_job_request = abi_encode_offchain_job_request(job).unwrap();

        let private_key_hex = env::var("OFFCHAIN_SIGNER_PRIVATE_KEY")
            .expect("OFFCHAIN_SIGNER_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_job_request).await.unwrap();

        println!("Encoded job request: {}", hex::encode(&encoded_job_request));
        println!("Signature for encoded job request: {}", hex::encode(signature.as_bytes()));
    }
}

type AddressWithBalance = sol! {
    tuple(address,uint256)
};

fn abi_encode_address_with_balance(address: Address, balance: U256) -> Vec<u8> {
    AddressWithBalance::abi_encode(&(address, balance))
}
