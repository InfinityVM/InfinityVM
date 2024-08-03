use alloy::{
    primitives::{hex, keccak256, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
};
use alloy_sol_types::{sol, SolType};
use k256::ecdsa::SigningKey;
use proto::JobInputs;
use zkvm_executor::service::abi_encode_result_with_metadata;

type K256LocalSigner = LocalSigner<SigningKey>;

/// Script to generate ABI-encoded responses + signatures for the coprocessor contract tests
#[derive(Debug)]
pub struct RequestAndResultSigner;

impl RequestAndResultSigner {
    /// Sign a result for a job requested onchain
    pub async fn sign_onchain_result() {
        let zero_addr_str = "0x0000000000000000000000000000000000000000";
        let zero_addr: Address = Address::parse_checksummed(zero_addr_str, None).unwrap();

        let job_inputs = JobInputs {
            job_id: 1,
            program_input: abi_encode_address(zero_addr),
            max_cycles: 1_000_000,
            program_verifying_key: b"programID".to_vec(),
            program_elf: b"elf".to_vec(),
            vm_type: 0,
        };

        // Encode the result with metadata
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_result = abi_encode_result_with_metadata(&job_inputs, &raw_output);

        // Sign the message
        let decoded = hex::decode("").unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_result).await.unwrap();

        println!("Encoded onchain result: {}", hex::encode(&encoded_result));
        println!("Signature for encoded onchain result: {}", hex::encode(signature.as_bytes()));
    }

    /// Sign an offchain job request
    pub async fn sign_job_request() {
        let zero_addr_str = "0x0000000000000000000000000000000000000000";
        let zero_addr: Address = Address::parse_checksummed(zero_addr_str, None).unwrap();

        let consumer_addr_str = "0xDB8cFf278adCCF9E9b5da745B44E754fC4EE3C76";
        let consumer_addr: Address = Address::parse_checksummed(consumer_addr_str, None).unwrap();

        let encoded_job_request = abi_encode_job_request(
            1,
            1_000_000,
            consumer_addr,
            b"programID".to_vec(),
            abi_encode_address(zero_addr),
        );

        let decoded = hex::decode("").unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_job_request).await.unwrap();

        println!("Encoded job request: {}", hex::encode(&encoded_job_request));
        println!("Signature for encoded job request: {}", hex::encode(signature.as_bytes()));
    }

    /// Sign a result for a job requested offchain
    pub async fn sign_offchain_result() {
        let zero_addr_str = "0x0000000000000000000000000000000000000000";
        let zero_addr: Address = Address::parse_checksummed(zero_addr_str, None).unwrap();

        let program_input_hash = keccak256(abi_encode_address(zero_addr));
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_offchain_result = abi_encode_offchain_result(
            program_input_hash.into(),
            1_000_000,
            b"programID".to_vec(),
            raw_output,
        );

        // Sign the message
        let decoded = hex::decode("").unwrap(); // Replace with your actual private key
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

type JobRequest = sol! {
    tuple(uint32,uint64,address,bytes,bytes)
};

fn abi_encode_job_request(
    nonce: u32,
    max_cycles: u64,
    consumer: Address,
    program_id: Vec<u8>,
    program_input: Vec<u8>,
) -> Vec<u8> {
    JobRequest::abi_encode_params(&(nonce, max_cycles, consumer, program_id, program_input))
}

type OffchainResult = sol! {
    tuple(bytes32,uint64,bytes,bytes)
};

fn abi_encode_offchain_result(
    program_input_hash: [u8; 32],
    max_cycles: u64,
    program_id: Vec<u8>,
    result: Vec<u8>,
) -> Vec<u8> {
    OffchainResult::abi_encode_params(&(program_input_hash, max_cycles, program_id, result))
}
