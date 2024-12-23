use crate::remote_db::RemoteElfClientTrait;
use ivm_db::tables::ElfWithMeta;
use mockall::automock;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct MockRemoteElfClient {
    pub stored_elfs: Arc<Mutex<Vec<(Vec<u8>, ElfWithMeta)>>>,
}

impl MockRemoteElfClient {
    pub fn new() -> Self {
        Self { stored_elfs: Arc::new(Mutex::new(Vec::new())) }
    }

    pub async fn add_elf(&self, program_id: Vec<u8>, elf: ElfWithMeta) {
        let mut elfs = self.stored_elfs.lock().await;
        elfs.push((program_id, elf));
    }
}

#[automock]
#[async_trait::async_trait]
impl RemoteElfClientTrait for MockRemoteElfClient {
    async fn get_elf(&mut self, program_id: Vec<u8>) -> Result<ElfWithMeta, tonic::Status> {
        let elfs = self.stored_elfs.lock().await;
        if let Some((_, elf)) = elfs.iter().find(|(pid, _)| pid == &program_id) {
            Ok(elf.clone())
        } else {
            Err(tonic::Status::not_found("ELF not found"))
        }
    }

    async fn store_elf(
        &mut self,
        program_id: Vec<u8>,
        elf: ElfWithMeta,
    ) -> Result<(), tonic::Status> {
        let mut elfs = self.stored_elfs.lock().await;
        elfs.push((program_id, elf));
        Ok(())
    }
}
