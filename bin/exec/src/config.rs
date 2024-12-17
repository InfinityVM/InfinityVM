//! IVM specific configuration definition and IO helpers.

use crate::pool::validator::IvmTransactionAllowConfig;
use eyre::eyre;
use std::{fs, path::Path};

/// IVM specific configuration for the execution client.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IvmConfig {
    /// Configuration for allow list based on sender and recipient.
    pub transaction_allow: IvmTransactionAllowConfig,
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

#[cfg(test)]
mod test {
    use super::IvmConfig;
    use alloy::primitives::address;
    use std::collections::HashSet;

    #[test]
    fn read_in_existing_config() {
        let config = IvmConfig::from_path("./mock/ivm_config.toml").unwrap();

        let to = HashSet::from([address!("1111111111111111111111111111111111111111")]);
        let sender = HashSet::from([
            address!("2222222222222222222222222222222222222222"),
            address!("1234567890123456789012345678901234567890"),
        ]);

        assert_eq!(config.transaction_allow.to(), to);
        assert_eq!(config.transaction_allow.sender(), sender);
        assert!(!config.transaction_allow.all());
    }

    #[test]
    fn creates_default_config_when_non_existent() {
        let path = "./this-should-not-exist-mock-config.toml";
        let config = IvmConfig::from_path(path).unwrap();

        assert_eq!(config.transaction_allow.to(), HashSet::new());
        assert_eq!(config.transaction_allow.sender(), HashSet::new());
        // We expect it to default to allowing everything
        assert!(config.transaction_allow.all());

        // This will generate a file, so we need to remove it
        std::fs::remove_file(path).unwrap();
    }
}
