//! IVM specific configuration definition and IO helpers.

use std::fs;
use std::path::Path;
use eyre::eyre;
use std::collections::HashSet;
use alloy::primitives::Address;

/// IVM specific configuration for the execution client.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IvmConfig {
    /// Allow list of senders for transactions.
    pub tx_allowed_sender: HashSet<Address>,
    /// Allow list of `to` field value for transaction.
    pub tx_allowed_to: HashSet<Address>,
    /// Allow all transactions, overrides allow lists.
    pub tx_allow_all: bool,
}

impl Default for IvmConfig {
    fn default() -> Self {
        Self {
            tx_allowed_sender: HashSet::default(),
            tx_allowed_to: HashSet::default(),
            tx_allow_all: true,
        }
    }
}

impl IvmConfig {
    /// Load a [`IvmConfig`] from a specified path.
    ///
    /// A new configuration file is created with default values if none
    /// exists.
    pub fn from_path(path: impl AsRef<Path>) -> eyre::Result<Self> {
        let path = path.as_ref();

        match fs::read_to_string(path) {
            Ok(cfg_string) => {
                toml::from_str(&cfg_string).map_err(|e| eyre!("failed to parse TOML: {e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| eyre!("failed to create directory: {e}"))?;
                }
                let cfg = Self::default();
                let s = toml::to_string_pretty(&cfg)
                    .map_err(|e| eyre!("failed to serialize to TOML: {e}"))?;
                fs::write(path, s).map_err(|e| eyre!("failed to write configuration file: {e}"))?;
                Ok(cfg)
            }
            Err(e) => Err(eyre!("failed to load configuration: {e}")),
        }
    }
}