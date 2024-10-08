//! Generate coprocessor node types based off of protobuf definitions.
//! 
//! The generated code is part of the `proto` crate/

use std::path::PathBuf;

const SERDE_SER_DER_DERIVE: &str = "#[derive(serde::Serialize, serde::Deserialize)]";
const SERDE_AS: &str = "#[serde_as]";
const BORSH_SER_DER_DERIVE: &str = "#[derive(borsh::BorshSerialize, borsh::BorshDeserialize,)]";
const SERDE_RENAME_CAMELCASE: &str = "#[serde(rename_all = \"camelCase\")]";
const SERDE_BYTES_HEX: &str = "#[serde_as(as = \"Hex\")]";
const BORSH_USE_DISCRIMINANT_TRUE: &str = "#[borsh(use_discriminant=true)]";
const SERDE_DEFAULT: &str = "#[serde(default)]";
const PROTO_CRATE_SRC: &str = "crates/sdk/proto/src";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(PROTO_CRATE_SRC);

    tonic_build::configure()
        .type_attribute(".coprocessor_node", SERDE_AS)
        .type_attribute(".coprocessor_node", SERDE_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node", SERDE_RENAME_CAMELCASE)
        .type_attribute(".coprocessor_node.v1.JobStatus", BORSH_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node.v1.JobStatusType", BORSH_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node.v1.JobStatusType", BORSH_USE_DISCRIMINANT_TRUE)
        .field_attribute(".coprocessor_node.v1.JobStatus.failure_reason", SERDE_DEFAULT)
        .field_attribute(".coprocessor_node.v1.JobStatus.retries", SERDE_DEFAULT)
        .field_attribute("id", SERDE_BYTES_HEX)
        .field_attribute("program_elf", SERDE_BYTES_HEX)
        .field_attribute("program_id", SERDE_BYTES_HEX)
        .field_attribute("onchain_input", SERDE_BYTES_HEX)
        .field_attribute("offchain_input_hash", SERDE_BYTES_HEX)
        .field_attribute("state_hash", SERDE_BYTES_HEX)
        .field_attribute("request_signature", SERDE_BYTES_HEX)
        .field_attribute("result_with_metadata", SERDE_BYTES_HEX)
        .field_attribute("zkvm_operator_signature", SERDE_BYTES_HEX)
        .field_attribute("consumer_address", SERDE_BYTES_HEX)
        .field_attribute("relay_tx_hash", SERDE_BYTES_HEX)
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .out_dir(out_dir)
        .compile(
            &[
                "proto/coprocessor_node/v1/coprocessor_node.proto",
                "proto/coprocessor_node/v1/job.proto",
            ],
            &["proto"],
        )
        .unwrap();

    Ok(())
}
