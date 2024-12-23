//! Configuration for the coprocessor node.

use std::net::SocketAddr;

/// Configuration for remote ELF store.
#[derive(Debug, Clone)]
pub struct RemoteDbConfig {
    /// Remote DB endpoint (e.g., "http://localhost:50051")
    pub endpoint: String,
}

impl Default for RemoteDbConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051".to_string(),
        }
    }
}

/// Configuration for the coprocessor node.
#[derive(Debug, Clone)]
pub struct Config {
    /// Remote DB configuration
    pub remote_db: RemoteDbConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            remote_db: RemoteDbConfig::default(),
        }
    }
}
