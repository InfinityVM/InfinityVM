//! Compile the protobufs. We can make this more complex by doing things like
//! - derives (Eq, Default etc)
//! - write the output to files

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../../proto/server/v1/zkvm_executor.proto")?;

    Ok(())
}
