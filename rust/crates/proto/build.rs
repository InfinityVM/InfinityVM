//! Compile the protobufs. We can make this more complex by doing things like
//! - derives (Eq, Default etc)
//! - write the output to files

use std::{env, path::PathBuf};

const SERDE_SER_DER_DERIVE: &str = "#[derive(serde::Serialize, serde::Deserialize)]";
const SERDE_AS: &str = "#[serde_as]";
const BORSH_SER_DER_DERIVE: &str = "#[derive(borsh::BorshSerialize, borsh::BorshDeserialize,)]";
const SERDE_RENAME_CAMELCASE: &str = "#[serde(rename_all = \"camelCase\")]";
const SERDE_BYTES_BASE64: &str = "#[serde_as(as = \"Base64\")]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .type_attribute(".coprocessor_node", SERDE_AS)
        .type_attribute(".coprocessor_node", SERDE_SER_DER_DERIVE)
        .type_attribute(".coprocessor_node", SERDE_RENAME_CAMELCASE)
        .type_attribute("Job", BORSH_SER_DER_DERIVE)
        .field_attribute("program_elf", SERDE_BYTES_BASE64)
        .field_attribute("program_verifying_key", SERDE_BYTES_BASE64)
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile(
            &[
                "../../../proto/coprocessor_node/v1/zkvm_executor.proto",
                "../../../proto/coprocessor_node/v1/coprocessor_node.proto",
                "../../../proto/coprocessor_node/v1/job.proto",
            ],
            &["../../../proto"],
        )
        .unwrap();

    Ok(())
}
