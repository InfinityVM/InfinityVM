use alloy::{
    primitives::{hex, Address, Uint, U256},
    signers::{local::LocalSigner, Signer},
};
use alloy_sol_types::{sol, SolType};
use executor::service::abi_encode_result_with_metadata;
use k256::ecdsa::SigningKey;
use proto::JobInputs;

type K256LocalSigner = LocalSigner<SigningKey>;

/// Script to generate ABI-encoded response + signature for the coprocessor contract tests
#[derive(Debug)]
pub struct ResultSigner;

impl ResultSigner {
    /// Run the `ResultSigner`
    pub async fn run() {
        let zero_addr_str = "0x0000000000000000000000000000000000000000";
        let zero_addr: Address = Address::parse_checksummed(zero_addr_str, None).unwrap();

        let job_inputs = JobInputs {
            job_id: 1,
            program_input: abi_encode_address(zero_addr),
            max_cycles: 1_000_000,
            program_verifying_key: b"programID".to_vec(),
            vm_type: 0,
        };

        // Encode the result with metadata
        let raw_output = abi_encode_address_with_balance(zero_addr, Uint::from(10));
        let encoded_data = abi_encode_result_with_metadata(&job_inputs, &raw_output);

        // Sign the message
        let decoded = hex::decode("").unwrap(); // Replace with your actual private key
        let signer = K256LocalSigner::from_slice(&decoded).unwrap();
        let signature = signer.sign_message(&encoded_data).await.unwrap();

        println!("Encoded Data: {}", hex::encode(&encoded_data));
        println!("Signature: {}", hex::encode(signature.as_bytes()));
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
