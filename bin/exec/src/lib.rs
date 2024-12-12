//! IVM execution client types for plugging into reth node builder.

use crate::engine::IvmEngineValidatorBuilder;
use reth::{
    builder::{rpc::RpcAddOns, FullNodeComponents, FullNodeTypes},
    network::NetworkHandle,
    rpc::eth::EthApi,
};

pub mod engine;
pub mod pool;

/// RPC add on that includes [`IvmEngineValidatorBuilder`].
pub type IvmAddOns<N> = RpcAddOns<
    N,
    EthApi<
        <N as FullNodeTypes>::Provider,
        <N as FullNodeComponents>::Pool,
        NetworkHandle,
        <N as FullNodeComponents>::Evm,
    >,
    IvmEngineValidatorBuilder,
>;

/// Add arguments to the default reth cli.
#[derive(Debug, Clone, Copy, Default, clap::Args)]
struct RethCliExt {
    /// CLI flag to pass a path for the ivm specific config file path
    #[arg(long)]
    pub ivm_config: PathBuf,
}

#[derive(serde::Serialize, serde:Deserialize)]
struct IvmConfig {
    allowed_sender: HashSet<Address>,
    allowed_to: HashSet<Address>
}

impl Default for IvmConfig {
    fn default() -> Self {
        // TODO: figure out what we want to do for defaults
        allowed_sender: HashSet::default(),
        allowed_to: HashSet::default(),
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
                toml::from_str(&cfg_string).map_err(|e| eyre!("Failed to parse TOML: {e}"))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| eyre!("Failed to create directory: {e}"))?;
                }
                let cfg = Self::default();
                let s = toml::to_string_pretty(&cfg)
                    .map_err(|e| eyre!("Failed to serialize to TOML: {e}"))?;
                fs::write(path, s).map_err(|e| eyre!("Failed to write configuration file: {e}"))?;
                Ok(cfg)
            }
            Err(e) => Err(eyre!("Failed to load configuration: {e}")),
        }
    }
}