use alloy::{
    primitives::{hex, keccak256, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
    sol,
    sol_types::SolType,
};
use coprocessor_node::job_processor::abi_encode_offchain_job_request;
use dotenv::dotenv;
use k256::ecdsa::SigningKey;
use proto::{Job, JobInputs, RequestType};
use std::env;
use test_utils::get_job_id;
use zkvm_executor::service::{
    abi_encode_offchain_result_with_metadata, abi_encode_result_with_metadata,
};

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
            program_input: abi_encode_address(zero_addr),
            max_cycles: MAX_CYCLES,
            program_verifying_key: PROGRAM_ID.to_vec(),
            program_elf: PROGRAM_ELF.to_vec(),
            vm_type: VM_TYPE,
            request_type: RequestType::Onchain as i32,
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
            consumer_address: abi_encode_address(consumer_addr),
            program_verifying_key: PROGRAM_ID.to_vec(),
            input: abi_encode_address(zero_addr),
            request_signature: vec![],
            result: vec![],
            zkvm_operator_address: vec![],
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

    /// Sign a result for a job requested offchain
    pub async fn sign_offchain_result() {
        dotenv().ok();

        let zero_addr: Address = Address::ZERO;

        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let job_inputs = JobInputs {
            job_id: vec![],
            program_input: abi_encode_address(zero_addr),
            max_cycles: MAX_CYCLES,
            program_verifying_key: PROGRAM_ID.to_vec(),
            program_elf: PROGRAM_ELF.to_vec(),
            vm_type: VM_TYPE,
            request_type: RequestType::Onchain as i32,
        };
        let encoded_offchain_result =
            abi_encode_offchain_result_with_metadata(&job_inputs, &raw_output).unwrap();

        // Sign the message
        let private_key_hex = env::var("COPROCESSOR_OPERATOR_PRIVATE_KEY")
            .expect("COPROCESSOR_OPERATOR_PRIVATE_KEY not set in .env file");
        let decoded = hex::decode(private_key_hex).unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_offchain_result).await.unwrap();

        println!("Encoded offchain result: {}", hex::encode(&encoded_offchain_result));
        println!("Signature for encoded offchain result: {}", hex::encode(signature.as_bytes()));
    }
}

type AddressEncodeable = sol! {
    address
};

fn abi_encode_address(address: Address) -> Vec<u8> {
    AddressEncodeable::abi_encode(&address)
}

type AddressWithBalance = sol! {
    tuple(address,uint256)
};

fn abi_encode_address_with_balance(address: Address, balance: U256) -> Vec<u8> {
    AddressWithBalance::abi_encode(&(address, balance))
}
