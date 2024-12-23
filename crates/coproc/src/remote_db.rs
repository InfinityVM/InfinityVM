//! Remote DB client for ELF storage and retrieval.

use ivm_remote_db::remote_db::v1::{
    elf_store_client::ElfStoreClient, GetElfRequest, StoreElfRequest,
};
use tonic::transport::Channel;
use tracing::{debug, warn};

/// Client for interacting with the remote ELF store.
#[derive(Debug)]
pub struct RemoteElfClient {
    client: ElfStoreClient<Channel>,
}

impl RemoteElfClient {
    /// Create a new RemoteElfClient connected to the specified endpoint.
    pub async fn connect(endpoint: &str) -> Result<Self, tonic::transport::Error> {
        debug!("Connecting to remote ELF store at {}", endpoint);
        let client = ElfStoreClient::connect(endpoint.to_string()).await?;
        Ok(Self { client })
    }

    /// Store an ELF program in the remote database.
    pub async fn store_elf(
        &mut self,
        program_id: Vec<u8>,
        program_elf: Vec<u8>,
        vm_type: i32,
    ) -> Result<bool, tonic::Status> {
        debug!("Storing ELF in remote DB");
        let request = tonic::Request::new(StoreElfRequest { program_id, program_elf, vm_type });

        match self.client.store_elf(request).await {
            Ok(response) => {
                let success = response.into_inner().success;
                if !success {
                    warn!("Remote DB reported unsuccessful ELF storage");
                }
                Ok(success)
            }
            Err(e) => {
                warn!("Failed to store ELF in remote DB: {}", e);
                Err(e)
            }
        }
    }

    /// Retrieve an ELF program from the remote database by its program ID.
    pub async fn get_elf(
        &mut self,
        program_id: Vec<u8>,
    ) -> Result<Option<(Vec<u8>, i32)>, tonic::Status> {
        debug!("Retrieving ELF from remote DB");
        let request = tonic::Request::new(GetElfRequest { program_id });

        match self.client.get_elf(request).await {
            Ok(response) => {
                let response = response.into_inner();
                Ok(Some((response.program_elf, response.vm_type)))
            }
            Err(status) if status.code() == tonic::Code::NotFound => {
                debug!("ELF not found in remote DB");
                Ok(None)
            }
            Err(e) => {
                warn!("Error retrieving ELF from remote DB: {}", e);
                Err(e)
            }
        }
    }
}
