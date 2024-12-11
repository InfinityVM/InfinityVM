// TODO: load from config

const EXTENSION: &str = "toml";

// struct IvmConfig;

// impl IvmConfig {
//     /// Load a [`Config`] from a specified path.
//     ///
//     /// A new configuration file is created with default values if none
//     /// exists.
//     pub fn from_path(path: impl AsRef<Path>) -> eyre::Result<Self> {
//         let path = path.as_ref();
//         match fs::read_to_string(path) {
//             Ok(cfg_string) => {
//                 toml::from_str(&cfg_string).map_err(|e| eyre!("Failed to parse TOML: {e}"))
//             }
//             Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
//                 if let Some(parent) = path.parent() {
//                     fs::create_dir_all(parent)
//                         .map_err(|e| eyre!("Failed to create directory: {e}"))?;
//                 }
//                 let cfg = Self::default();
//                 let s = toml::to_string_pretty(&cfg)
//                     .map_err(|e| eyre!("Failed to serialize to TOML: {e}"))?;
//                 fs::write(path, s).map_err(|e| eyre!("Failed to write configuration file: {e}"))?;
//                 Ok(cfg)
//             }
//             Err(e) => Err(eyre!("Failed to load configuration: {e}")),
//         }
//     }
// }