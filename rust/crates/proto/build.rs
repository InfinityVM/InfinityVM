//! Compile the protobufs. We can make this more complex by doing things like
//! - derives (Eq, Default etc)
//! - write the output to files

use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("zkvm_executor_descriptor.bin"))
        .compile(
            &["../../../proto/zkvm_executor/zkvm_executor.proto"],
            &["../../../proto/zkvm_executor/"],
        )
        .unwrap();

    Ok(())
}
