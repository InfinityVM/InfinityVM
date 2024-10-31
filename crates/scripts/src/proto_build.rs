//! Generate coprocessor node types based off of protobuf definitions.
//!
//! The generated code is part of the `proto` crate.

use std::path::PathBuf;

const SERDE_SER_DER_DERIVE: &str = "#[derive(serde::Serialize, serde::Deserialize)]";
const SERDE_AS: &str = "#[serde_as]";
const SERDE_RENAME_CAMELCASE: &str = "#[serde(rename_all = \"camelCase\")]";
const SERDE_DEFAULT: &str = "#[serde(default)]";
const PROTO_CRATE_SRC: &str = "crates/sdk/proto/src";
const SERDE_BYTES_HEX: &str = "#[serde_as(as = \"Hex\")]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(PROTO_CRATE_SRC);

    tonic_build::configure()
        .type_attribute(".coprocessor_node", SERDE_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node", SERDE_RENAME_CAMELCASE)
        .type_attribute(".coprocessor_node.v1.JobResult", SERDE_AS)
        .type_attribute(".coprocessor_node.v1.JobStatus", SERDE_AS)
        .field_attribute(".coprocessor_node.v1.JobStatus.failure_reason", SERDE_DEFAULT)
        .field_attribute(".coprocessor_node.v1.JobStatus.retries", SERDE_DEFAULT)
        .field_attribute(".coprocessor_node.v1.JobResult.id", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.program_elf", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.program_id", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.onchain_input", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.offchain_input_hash", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.request_signature", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.result_with_metadata", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.zkvm_operator_signature", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.consumer_address", SERDE_BYTES_HEX)
        .field_attribute(".coprocessor_node.v1.JobResult.relay_tx_hash", SERDE_BYTES_HEX)
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
