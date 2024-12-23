use remote_db::remote_db::v1::elf_store_client::ElfStoreClient;
use remote_db::remote_db::v1::{GetElfRequest, StoreElfRequest};
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ElfStoreClient::connect("http://[::1]:50052").await?;

    let elf_path = "target/sp1/intensity-test/intensity-test-sp1-guest";
    let program_id = b"intensity-test-sp1-guest".to_vec();
    let program_elf = fs::read(elf_path)?;

    println!("Storing ELF program from {}...", elf_path);
    println!("ELF size: {} bytes", program_elf.len());

    let response = client
        .store_elf(StoreElfRequest {
            program_id: program_id.clone(),
            program_elf: program_elf.clone(),
            vm_type: 1,
        })
        .await?;
    println!("Store response: {:?}", response);

    println!("\nRetrieving stored ELF...");
    let response = client.get_elf(GetElfRequest { program_id: program_id.clone() }).await?;
    println!("Retrieved ELF size: {} bytes", response.get_ref().program_elf.len());

    assert_eq!(
        response.get_ref().program_elf,
        program_elf,
        "Retrieved ELF doesn't match original!"
    );
    println!("✅ Retrieved ELF matches original file!");

    Ok(())
}
