/// Configuration for remote ELF store.
#[derive(Debug, Clone)]
pub struct RemoteDbConfig {
    /// Remote DB endpoint
    pub endpoint: String,
}

impl Default for RemoteDbConfig {
    fn default() -> Self {
        Self { endpoint: "http://127.0.0.1:50051".to_string() }
    }
}

/// Configuration for the coprocessor node.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Remote DB configuration
    pub elf_store: RemoteDbConfig,
}
