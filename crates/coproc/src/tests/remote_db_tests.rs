use super::mocks::{MockRemoteElfClient, MockRemoteElfClientTrait};
use crate::{
    config::Config,
    job_executor::JobExecutor,
    remote_db::RemoteElfClientTrait,
};
use ivm_db::tables::ElfWithMeta;
use mockall::predicate::*;
use std::sync::Arc;
use tokio::sync::mpsc::channel;

#[tokio::test]
async fn test_get_elf_from_remote_db() {
    // Setup mock client
    let mut mock_client = MockRemoteElfClient::new();
    
    // Create test data
    let program_id = vec![1, 2, 3, 4];
    let elf_data = vec![5, 6, 7, 8];
    let expected_elf = ElfWithMeta { vm_type: 1, elf: elf_data.clone() };

    // Add test ELF to mock client
    mock_client.add_elf(program_id.clone(), expected_elf.clone()).await;

    // Test get_elf
    let result = mock_client.get_elf(program_id.clone()).await;
    
    // Verify results
    assert!(result.is_ok(), "Failed to get ELF from remote DB");
    if let Ok(elf) = result {
        assert_eq!(elf.elf, expected_elf.elf);
        assert_eq!(elf.vm_type, expected_elf.vm_type);
    }
}

#[tokio::test]
async fn test_store_elf_in_remote_db() {
    // Setup mock client
    let mut mock_client = MockRemoteElfClient::new();
    
    // Create test data
    let program_id = vec![1, 2, 3, 4];
    let elf_data = vec![5, 6, 7, 8];
    let test_elf = ElfWithMeta { vm_type: 1, elf: elf_data.clone() };

    // Test store_elf
    let store_result = mock_client.store_elf(program_id.clone(), test_elf.clone()).await;
    assert!(store_result.is_ok(), "Failed to store ELF in remote DB");

    // Verify ELF was stored by retrieving it
    let get_result = mock_client.get_elf(program_id).await;
    assert!(get_result.is_ok(), "Failed to get stored ELF from remote DB");
    if let Ok(elf) = get_result {
        assert_eq!(elf.elf, test_elf.elf);
        assert_eq!(elf.vm_type, test_elf.vm_type);
    }
}

#[tokio::test]
async fn test_get_nonexistent_elf() {
    // Setup mock client
    let mut mock_client = MockRemoteElfClient::new();
    
    // Try to get non-existent ELF
    let result = mock_client.get_elf(vec![99, 99, 99, 99]).await;
    
    // Verify error response
    assert!(result.is_err(), "Expected error when getting non-existent ELF");
    assert_eq!(
        result.unwrap_err().code(),
        tonic::Code::NotFound,
        "Expected NotFound error for non-existent ELF"
    );
}

#[tokio::test]
async fn test_job_executor_remote_db_integration() {
    // Setup
    let (exec_queue_sender, _) = channel(1);
    let (writer_tx, _) = channel(1);
    let db = Arc::new(MockDb::default());
    let metrics = Arc::new(crate::metrics::Metrics::new());
    let config = Config::default();
    let zk_executor = MockExecutor::default();

    // Create test data
    let program_id = vec![1, 2, 3, 4];
    let elf_data = vec![5, 6, 7, 8];
    let test_elf = ElfWithMeta { vm_type: 1, elf: elf_data.clone() };

    // Create mock job
    let mut job = ivm_db::tables::Job { program_id: program_id.clone(), ..Default::default() };

    // Create JobExecutor
    let executor = JobExecutor::new(
        db.clone(),
        exec_queue_sender,
        zk_executor,
        metrics.clone(),
        writer_tx.clone(),
        config.clone(),
    );

    // Test ELF retrieval when not in local DB
    let result = executor.get_elf(&mut job).await;
    assert!(result.is_err(), "Expected error when ELF not in local or remote DB");

    // Add ELF to remote DB
    let mut remote_client = MockRemoteElfClient::new();
    remote_client.add_elf(program_id.clone(), test_elf.clone()).await;

    // Test ELF retrieval from remote DB
    let result = executor.get_elf(&mut job).await;
    assert!(result.is_ok(), "Failed to get ELF from remote DB");
    if let Ok(elf) = result {
        assert_eq!(elf.elf, test_elf.elf);
        assert_eq!(elf.vm_type, test_elf.vm_type);
    }
}
