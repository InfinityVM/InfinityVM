//! Compile the protobufs. We can make this more complex by doing things like
//! - derives (Eq, Default etc)
//! - write the output to files

use std::{env, path::PathBuf};

const SERDE_SER_DER_DERIVE: &str = "#[derive(serde::Serialize, serde::Deserialize)]";
const SERDE_AS: &str = "#[serde_as]";
const BORSH_SER_DER_DERIVE: &str = "#[derive(borsh::BorshSerialize, borsh::BorshDeserialize,)]";
const SERDE_RENAME_CAMELCASE: &str = "#[serde(rename_all = \"camelCase\")]";
const SERDE_BYTES_BASE64: &str = "#[serde_as(as = \"Base64\")]";
const SERDE_BYTES_HEX: &str = "#[serde_as(as = \"Hex\")]";
const BORSH_USE_DISCRIMINANT_TRUE: &str = "#[borsh(use_discriminant=true)]";
const SERDE_DEFAULT: &str = "#[serde(default)]"; // New constant for serde default

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .type_attribute(".coprocessor_node", SERDE_AS)
        .type_attribute(".coprocessor_node", SERDE_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node", SERDE_RENAME_CAMELCASE)
        .type_attribute(".coprocessor_node.v1.Job", BORSH_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node.v1.JobStatus", BORSH_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node.v1.JobStatusType", BORSH_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node.v1.JobStatusType", BORSH_USE_DISCRIMINANT_TRUE)
        .field_attribute(".coprocessor_node.v1.JobStatus.failure_reason", SERDE_DEFAULT)
        .field_attribute(".coprocessor_node.v1.JobStatus.retries", SERDE_DEFAULT)
        .field_attribute("id", SERDE_BYTES_BASE64)
        .field_attribute("program_elf", SERDE_BYTES_BASE64)
        .field_attribute("program_verifying_key", SERDE_BYTES_BASE64)
        .field_attribute("input", SERDE_BYTES_BASE64)
        .field_attribute("request_signature", SERDE_BYTES_BASE64)
        .field_attribute("result", SERDE_BYTES_BASE64)
        .field_attribute("zkvm_operator_signature", SERDE_BYTES_BASE64)
        .field_attribute("zkvm_operator_address", SERDE_BYTES_HEX)
        .field_attribute("contract_address", SERDE_BYTES_HEX)
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile(
            &[
                "../../proto/coprocessor_node/v1/zkvm_executor.proto",
                "../../proto/coprocessor_node/v1/coprocessor_node.proto",
                "../../proto/coprocessor_node/v1/job.proto",
            ],
            &["../../proto"],
        )
        .unwrap();

    Ok(())
}
