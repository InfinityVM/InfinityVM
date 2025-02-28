//! IVM specific configuration definition and IO helpers.

use crate::Network;
use alloy_primitives::{
    address,
    map::foldhash::{HashSet, HashSetExt},
    Address,
};
use eyre::eyre;
use std::{collections::BTreeMap, fs, path::Path};
use transaction::IvmTransactionAllowConfig;

pub mod transaction;

/// IVM specific configuration for the execution client. This is the in memory representation.
///
/// N.B. The default will allow deny all transactions.
///
/// We store persist this to disk with `IvmConfigToml` because map keys in toml must be strings,
/// but we use u64 keys.
#[derive(Debug, Clone)]
pub struct IvmConfig {
    /// Map from block timestamp to `IvmTransactionAllowConfig`
    forks: BTreeMap<u64, IvmTransactionAllowConfig>,
    /// Senders that are allowed in the forks. This is used for determining transaction priority.
    priority_senders: HashSet<Address>,
}

impl From<IvmConfigToml> for IvmConfig {
    fn from(toml: IvmConfigToml) -> Self {
        let forks: BTreeMap<_, _> = toml
            .forks
            .into_iter()
            .map(|IvmConfigForkToml { activation_timestamp, allow_config }| {
                (activation_timestamp, allow_config)
            })
            .collect();

        Self { forks, priority_senders: toml.priority_senders }
    }
}

impl Default for IvmConfig {
    fn default() -> Self {
        Self::deny_all()
    }
}

impl IvmConfig {
    /// If the transaction passes allow list checks at the fork associated with the
    /// given timestamp.
    ///
    /// Special case:
    /// - For a zero timestamp we always return true. We do this because at this timestamp no
    ///   consensus events have been processed by the engine.
    /// - If there are no forks and the timestamp is non-zero, we return false.
    pub fn is_allowed(&self, sender: &Address, to: Option<Address>, timestamp: u64) -> bool {
        if timestamp == 0 {
            return true
        };

        self.forks
            .range(..=timestamp)
            .next_back()
            .map(|(_, c)| {
                tracing::trace!(
                    allow_config=?c, ?timestamp, ?sender, ?to, "Selected allow config"
                );
                c.is_allowed(sender, to)
            })
            // Default to false if nothing is found
            .unwrap_or(false)
    }

    /// Return true if this sender is specified in the config as a priority sender.
    pub fn is_priority_sender(&self, sender: &Address) -> bool {
        self.priority_senders.contains(sender)
    }

    /// A config that allows all transactions.
    pub fn allow_all() -> Self {
        Self {
            forks: BTreeMap::from([(0, IvmTransactionAllowConfig::allow_all())]),
            priority_senders: HashSet::default(),
        }
    }

    /// A config that denies all transactions and has no priority senders.
    pub fn deny_all() -> Self {
        Self { forks: Default::default(), priority_senders: Default::default() }
    }

    /// Set a new fork at the given timestamp.
    #[cfg(any(feature = "test-utils", test))]
    pub fn set_fork(&mut self, timestamp: u64, allow_config: IvmTransactionAllowConfig) {
        self.forks.insert(timestamp, allow_config);
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
                let cfg: IvmConfigToml =
                    toml::from_str(&cfg_string).map_err(|e| eyre!("failed to parse TOML: {e}"))?;
                Ok(cfg.into())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| eyre!("failed to create directory: {e}"))?;
                }

                let cfg = Self::default();
                let toml: IvmConfigToml = cfg.clone().into();

                let s = toml::to_string_pretty(&toml)
                    .map_err(|e| eyre!("failed to serialize to TOML: {e}"))?;
                fs::write(path, s).map_err(|e| eyre!("failed to write configuration file: {e}"))?;
                Ok(cfg)
            }
            Err(e) => Err(eyre!("failed to load configuration: {e}")),
        }
    }

    /// Get the config from a particular network
    pub fn from_network(network: Network) -> Self {
        match network {
            Network::Suzuka => {
                let addresses_sender0 = vec![
                    address!("0x20f33ce90a13a4b5e7697e3544c3083b8f8a51d4"), // depositor
                    address!("0xf8bbb414ff7f6d8061cb16b7c23685300b6b5527"), // coproc-operator
                    address!("0x367457e0f4ae06bd090795ebdf585e04f6ee2c7f"), // coproc-relayer
                    address!("0x0a065cedfe7d1b9435152dde846aa1f2f4b49776"), // deployer
                    address!("0xa3354f9024f4dea7199bc944b0f27d95ea6295b1"), // faucet
                    address!("0x51e163d5fd1d98fa714241e8d60e2bf5c6166ebb"), // fee_recipient
                    address!("0x24e3ae70c81a16ac3fe5674e0556b769ea611cb6"), // fuzzer
                    address!("0x100bbbad1a19b680d84e5c1370c21dd637d8a6ce"), // gtx-admin
                    address!("0xbb0b808cce0c4f780bdfce67674eb1d6d24c7e12"), // offchain_signer
                    address!("0xf9fa39d99d3f7c62bf52966a20af60e673d45f3e"), // validator-0
                    address!("0xf791e9a8db4ab69caaf0179fa6ed808010277d96"), // validator-1
                    address!("0xcc1e4f683ab0eef001d2dd6c5c87c07a15c71f70"), // validator-2
                    address!("0xb6da9b28619412d728be31643ee10b1001c7d463"), // validator-3
                    address!("0x3361cddea38de48e38a78d0c8e147c4b068ee710"), /* fee recipient in
                                                                             * manifest */
                ];

                let addresses_to0 = vec![
                    address!("0x4242424242424242424242424242424242424242"), // Deposit contract
                    address!("0xf1508594b19bc7c19e7ddc0c38d0f6034fc7cc2b"), // mock sol
                    address!("0xe28da10f95837b327bd541ec8eea5cf9191a3f02"), // mock eth
                    address!("0xa45f78c667fd0d4857b52705f5d69eacc8d21f33"), // mock usdc
                    address!("0x0d558ed9bf39194b3addc46a686ea151b88758d7"), // mock usdt
                    address!("0xcb22256e10095506bff52f38362a09f7622fb6ce"), // gtx
                    address!("0x52e73d5849d368ee516dc136c418d265703b4065"), // job manager
                ];

                let mut fork0 = IvmTransactionAllowConfig::deny_all();
                let mut priority_senders = HashSet::new();
                for addr in addresses_sender0 {
                    fork0.add_sender(addr);
                    priority_senders.insert(addr);
                }
                for addr in addresses_to0 {
                    fork0.add_to(addr);
                }

                let forks = BTreeMap::from([
                    (0, fork0), // genesis
                ]);

                Self { forks, priority_senders }
            }
            Network::Monza => {
                // Privileged senders
                let addresses_sender0 = vec![
                    address!("0xe7a63aaf0afc13e559774917fc681ebabdf477ed"), // gtx-admin
                    address!("0x5bb4305ae8d6d227b8f2b4ec1e842c754ae4e089"), // coproc-relayer
                    address!("0xccd8368aa81d10ba1100d5bb6645bf5426424751"), // coproc-operator
                    address!("0xbb5430a58ef147a21f3362da6be68dfb70bc6d95"), // validator-0
                    address!("0x93c2266960a9b278a062f685e0a119c867614a38"), // validator-1
                    address!("0x71dfbca871aece51e0ea1f2b7d8c91872cc6627a"), // validator-2
                    address!("0xbe5a8f4bb624e69c4a08a7ebe14856e0f081aa7b"), // validator-3
                    address!("0x67f20d73130373fcf21eb165a095f79099d221a6"), // fee-recipient
                    address!("0x090d9e39939440a2c9a94040c61daff8b7c3b192"), // deployer
                    address!("0x2545ce5ae87283527a305fdc30a9c7167ca33dbc"), // faucet
                    address!("0x2a22352a44b79e7771a5bef021591e114622bebb"), // fuzzer
                    address!("0xf504f5f76a94145c75d04e495d307c4a88297966"), // offchain_signer
                    address!("0x20fe96e8783fba91acf07260b5fa40c3e655c5ff"), // validator-canary
                    address!("0x20f33ce90a13a4b5e7697e3544c3083b8f8a51d4"), // depositor
                ];

                // Deployed contracts
                let addresses_to0 = vec![
                    address!("0x4242424242424242424242424242424242424242"), // Deposit contract
                ];

                let mut fork0 = IvmTransactionAllowConfig::deny_all();
                let mut priority_senders = HashSet::new();
                for addr in addresses_sender0 {
                    fork0.add_sender(addr);
                    priority_senders.insert(addr);
                }
                for addr in addresses_to0 {
                    fork0.add_to(addr);
                }

                let forks = BTreeMap::from([
                    (0, fork0), // genesis
                ]);

                Self { forks, priority_senders }
            }
        }
    }
}

/// TOML representation of a single fork configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IvmConfigForkToml {
    activation_timestamp: u64,
    allow_config: IvmTransactionAllowConfig,
}

/// TOML representation of the complete IVM configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IvmConfigToml {
    forks: Vec<IvmConfigForkToml>,
    priority_senders: HashSet<Address>,
}

impl From<IvmConfig> for IvmConfigToml {
    fn from(ivm_config: IvmConfig) -> Self {
        let forks: Vec<_> = ivm_config
            .forks
            .into_iter()
            .map(|(activation_timestamp, allow_config)| IvmConfigForkToml {
                activation_timestamp,
                allow_config,
            })
            .collect();

        Self { forks, priority_senders: ivm_config.priority_senders }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::map::foldhash::HashSetExt;

    fn setup_test_config() -> IvmConfig {
        // Setup three forks at different timestamps
        let mut fork1 = IvmTransactionAllowConfig::deny_all();
        fork1.add_sender(Address::with_last_byte(1));

        let mut fork2 = IvmTransactionAllowConfig::deny_all();
        fork2.add_to(Address::with_last_byte(2));

        let fork3 = IvmTransactionAllowConfig::allow_all();

        let mut forks = BTreeMap::new();
        forks.insert(100, fork1);
        forks.insert(200, fork2);
        forks.insert(300, fork3);

        let mut priority_senders = HashSet::new();
        priority_senders.insert(Address::with_last_byte(3));

        IvmConfig { forks, priority_senders }
    }

    mod from_path {
        use std::str::FromStr;

        use super::*;
        use alloy_primitives::address;

        #[test]
        fn from_path_loads_correctly() {
            let config_file = "./mock/ivm_config.toml";
            let config = IvmConfig::from_path(config_file).unwrap();

            // Verify fork timestamps
            let fork_timestamps: Vec<_> = config.forks.keys().collect();
            let fork1_timestamp = 1672531200;
            let fork2_timestamp = 1688169600;
            let fork3_timestamp = 1704067200;
            assert_eq!(fork_timestamps, vec![&fork1_timestamp, &fork2_timestamp, &fork3_timestamp]);

            // First fork
            let fork1 = config.forks.get(&fork1_timestamp).unwrap();
            assert!(!fork1.all());
            assert_eq!(fork1.to().len(), 2);
            assert!(fork1.to().contains(&address!("0x1234567890123456789012345678901234567890")));
            assert!(fork1.to().contains(&address!("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd")));

            assert_eq!(fork1.sender().len(), 2);
            // Verify senders
            assert!(fork1
                .sender()
                .contains(&address!("0x2222222222222222222222222222222222222222")));
            assert!(fork1
                .sender()
                .contains(&address!("0x3333333333333333333333333333333333333333")));

            // Second fork
            let fork2 = config.forks.get(&fork2_timestamp).unwrap();
            assert_eq!(fork2.to().len(), 2);
            assert!(fork2.to().contains(&address!("0x4444444444444444444444444444444444444444")));
            assert!(fork2.to().contains(&address!("0x5555555555555555555555555555555555555555")));

            assert_eq!(fork2.sender().len(), 2);
            assert!(fork2
                .sender()
                .contains(&address!("0x6666666666666666666666666666666666666666")));
            assert!(fork2
                .sender()
                .contains(&address!("0x7777777777777777777777777777777777777777")));

            // Third fork
            let fork3 = config.forks.get(&fork3_timestamp).unwrap();
            assert!(fork3.all());
            assert!(fork3.to().is_empty());
            assert!(fork3.sender().is_empty());

            // Verify priority senders
            let expected_priority_senders: HashSet<Address> = [
                "0x8888888888888888888888888888888888888888",
                "0x9999999999999999999999999999999999999999",
                "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ]
            .iter()
            .map(|&s| Address::from_str(s).unwrap())
            .collect();

            assert_eq!(config.priority_senders, expected_priority_senders);

            // Verify all combinations of allowed transactions
            // First fork period
            assert!(config.is_allowed(
                &Address::with_last_byte(10),
                Some(address!("0x1234567890123456789012345678901234567890")),
                fork1_timestamp
            ));
            assert!(config.is_allowed(
                &address!("0x3333333333333333333333333333333333333333"),
                Some(Address::with_last_byte(10)),
                fork1_timestamp
            ));

            // Second fork period
            assert!(config.is_allowed(
                &Address::with_last_byte(10),
                Some(address!("0x4444444444444444444444444444444444444444")),
                fork2_timestamp
            ));
            assert!(config.is_allowed(
                &address!("0x7777777777777777777777777777777777777777"),
                Some(Address::with_last_byte(10)),
                fork2_timestamp
            ));

            // Third fork period - should allow all
            assert!(config.is_allowed(
                &Address::with_last_byte(10),
                Some(Address::with_last_byte(10)),
                fork3_timestamp
            ));

            // Verify non-allowed combinations
            // First fork period
            assert!(!config.is_allowed(
                &address!("0x1234567890123456789012345678901234567890"),
                Some(Address::with_last_byte(10)),
                fork1_timestamp
            ));

            // Second fork period
            assert!(!config.is_allowed(
                &Address::with_last_byte(10),
                Some(address!("0x6666666666666666666666666666666666666666")),
                fork2_timestamp
            ));

            // Verify priority sender functionality
            for sender in &expected_priority_senders {
                assert!(config.is_priority_sender(sender));
            }
            assert!(!config.is_priority_sender(&Address::with_last_byte(10)));
        }
    }

    mod is_allowed {
        use super::*;

        #[test]
        fn allows_by_sender() {
            let config = setup_test_config();
            let allowed_sender = Address::with_last_byte(1);
            let random_recipient = Address::with_last_byte(10);

            assert!(config.is_allowed(&allowed_sender, Some(random_recipient), 100));
            assert!(config.is_allowed(&allowed_sender, None, 150));
            assert!(!config.is_allowed(&random_recipient, None, 150));
        }

        #[test]
        fn allows_by_recipient() {
            let config = setup_test_config();
            let allowed_recipient = Address::with_last_byte(2);
            let random_sender = Address::with_last_byte(10);

            assert!(config.is_allowed(&random_sender, Some(allowed_recipient), 200));
            assert!(!config.is_allowed(&random_sender, Some(Address::with_last_byte(3)), 200));
        }

        #[test]
        fn handles_timestamp_transitions() {
            let config = setup_test_config();
            let sender = Address::with_last_byte(1);
            let recipient = Address::with_last_byte(2);
            let random_address = Address::with_last_byte(10);

            // Before first fork - deny all

            assert!(!config.is_allowed(&random_address, Some(random_address), 50));

            // At first fork - only allowed sender
            assert!(config.is_allowed(&sender, Some(random_address), 100));
            assert!(!config.is_allowed(&random_address, Some(random_address), 100));

            // first fork < timestamp < second fork - only allowed sender
            assert!(config.is_allowed(&sender, Some(random_address), 101));
            assert!(!config.is_allowed(&random_address, Some(recipient), 101));

            // At second fork - only allowed recipient
            assert!(config.is_allowed(&random_address, Some(recipient), 200));
            assert!(!config.is_allowed(&sender, Some(random_address), 200));

            // second fork < timestamp < third fork - only allowed recipient
            assert!(config.is_allowed(&random_address, Some(recipient), 299));
            assert!(!config.is_allowed(&sender, Some(random_address), 299));

            // At third fork - allow all
            assert!(config.is_allowed(&random_address, Some(random_address), 300));

            // third fork < timestamp
            assert!(config.is_allowed(&random_address, Some(random_address), 350));
        }

        #[test]
        fn handles_none_recipient() {
            let config = setup_test_config();
            let sender = Address::with_last_byte(1);

            // Should work with None recipient
            assert!(config.is_allowed(&sender, None, 100));
        }
    }

    mod priority_sender {
        use super::*;

        #[test]
        fn identifies_priority_senders() {
            let config = setup_test_config();

            let priority_sender = Address::with_last_byte(3);
            let normal_sender = Address::with_last_byte(1);

            assert!(config.is_priority_sender(&priority_sender));
            assert!(!config.is_priority_sender(&normal_sender));
        }

        #[test]
        fn handles_empty_priority_list() {
            let mut config = setup_test_config();
            config.priority_senders.clear();
            let former_priority_sender = Address::with_last_byte(3);

            assert!(!config.is_priority_sender(&former_priority_sender));
        }

        #[test]
        fn handles_multiple_priority_senders() {
            let mut config = setup_test_config();
            let sender1 = Address::with_last_byte(4);
            let sender2 = Address::with_last_byte(5);

            config.priority_senders.insert(sender1);
            config.priority_senders.insert(sender2);

            assert!(config.is_priority_sender(&sender1));
            assert!(config.is_priority_sender(&sender2));
        }
    }
}
