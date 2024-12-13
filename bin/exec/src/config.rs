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
    #[test]
    fn read_in_existing_config() {
        todo!()
    }

    #[test]
    fn creates_default_config_when_non_existent() {
        todo!()
    }
}
