fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/elf_store/v1/elf_store.proto")?;
    Ok(())
}
