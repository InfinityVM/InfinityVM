//! Compile the protobufs. We can make this more complex by doing things like
//! - derives (Eq, Default etc)
//! - write the output to files

use std::{env, path::PathBuf};

const SERDE_SER_DER_DERIVE: &str = "#[derive(serde::Serialize, serde::Deserialize)]";
const SERDE_RENAME_CAMELCASE: &str = "#[serde(rename_all = \"camelCase\")]";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .type_attribute(".server", SERDE_SER_DER_DERIVE)
        .type_attribute(".server", SERDE_RENAME_CAMELCASE)
        .file_descriptor_set_path(out_dir.join("descriptor.bin"))
        .compile(
            &[
                "../../../proto/server/v1/zkvm_executor.proto",
                "../../../proto/server/v1/service.proto",
                "../../../proto/server/v1/job.proto",
            ],
            &["../../../proto"],
        )
        .unwrap();

    Ok(())
}
