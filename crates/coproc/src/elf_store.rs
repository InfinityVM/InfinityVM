//! Remote DB client for ELF storage and retrieval.

use async_trait::async_trait;
use ivm_db::tables::ElfWithMeta;
use ivm_elf_store::elf_store::v1::{
    elf_store_client::ElfStoreClient, GetElfRequest, StoreElfRequest,
};
use tonic::transport::Channel;
use tracing::{debug, warn};

/// A client for interacting with the remote ELF storage service.
#[derive(Debug)]
pub struct RemoteElfClient {
    client: ElfStoreClient<Channel>,
}

impl RemoteElfClient {
    /// Create a new `RemoteElfClient` connected to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        debug!("Connecting to remote ELF store at {}", endpoint);
        let client = ElfStoreClient::connect(endpoint.to_string()).await?;
        Ok(Self { client })
    }
}

/// Trait defining the interface for remote ELF storage operations.
/// This trait is used to abstract the remote storage implementation and enable mocking in tests.
#[async_trait]
pub trait RemoteElfClientTrait {
    /// Retrieves an ELF binary and its metadata from remote storage by its program ID.
    /// Returns `Ok(ElfWithMeta)` if found, or a `tonic::Status` error if not found or on failure.
    async fn get_elf(&mut self, program_id: Vec<u8>) -> Result<ElfWithMeta, tonic::Status>;

    /// Stores an ELF binary and its metadata in remote storage.
    /// Returns `Ok(())` on success, or a `tonic::Status` error on failure.
    async fn store_elf(
        &mut self,
        program_id: Vec<u8>,
        elf: ElfWithMeta,
    ) -> Result<(), tonic::Status>;
}

#[async_trait]
impl RemoteElfClientTrait for RemoteElfClient {
    async fn get_elf(&mut self, program_id: Vec<u8>) -> Result<ElfWithMeta, tonic::Status> {
        debug!("Retrieving ELF from remote DB");
        let request = tonic::Request::new(GetElfRequest { program_id });

        match self.client.get_elf(request).await {
            Ok(response) => {
                let response = response.into_inner();
                Ok(ElfWithMeta { vm_type: response.vm_type as u8, elf: response.program_elf })
            }
            Err(e) => {
                warn!("Error retrieving ELF from remote DB: {}", e);
                Err(e)
            }
        }
    }

    async fn store_elf(
        &mut self,
        program_id: Vec<u8>,
        elf: ElfWithMeta,
    ) -> Result<(), tonic::Status> {
        debug!("Storing ELF in remote DB");
        let request = tonic::Request::new(StoreElfRequest {
            program_id,
            program_elf: elf.elf,
            vm_type: elf.vm_type as i32,
        });

        match self.client.store_elf(request).await {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Failed to store ELF in remote DB: {}", e);
                Err(e)
            }
        }
    }
}
