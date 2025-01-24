//! Allow list for transactions.

use alloy_primitives::{
    map::foldhash::{HashSet, HashSetExt},
    Address,
};

/// Configuration for allow list based on sender and recipient.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct IvmTransactionAllowConfig {
    all: bool,
    to: HashSet<Address>,
    sender: HashSet<Address>,
}

impl IvmTransactionAllowConfig {
    /// If the transaction passes allow list checks.
    #[inline(always)]
    pub fn is_allowed(&self, sender: &Address, to: Option<Address>) -> bool {
        if self.all {
            return true;
        }

        if let Some(to) = to {
            if self.to.contains(&to) {
                return true;
            }
        }

        self.sender.contains(sender)
    }

    /// Create a config that allows all transactions.
    pub fn allow_all() -> Self {
        Self { all: true, ..Default::default() }
    }

    /// Create `IvmTransactionAllowConfig` that denys all transactions.
    pub fn deny_all() -> Self {
        Self { to: HashSet::new(), sender: HashSet::new(), all: false }
    }

    /// Add a `to` address.
    #[cfg(any(feature = "test-utils", test))]
    pub fn add_to(&mut self, to: Address) {
        self.to.insert(to);
    }

    /// Get `to` addresses.
    #[cfg(any(feature = "test-utils", test))]
    pub fn to(&self) -> HashSet<Address> {
        self.to.clone()
    }

    /// Add a `sender` address.
    #[cfg(any(feature = "test-utils", test))]
    pub fn add_sender(&mut self, sender: Address) {
        self.sender.insert(sender);
    }

    /// Get allowed sender addresses.
    #[cfg(any(feature = "test-utils", test))]
    pub fn sender(&self) -> HashSet<Address> {
        self.sender.clone()
    }

    /// Allow all transactions
    #[cfg(any(feature = "test-utils", test))]
    pub fn set_all(&mut self, allow_all: bool) {
        self.all = allow_all;
    }

    /// Get if all transactions are allowed.
    #[cfg(any(feature = "test-utils", test))]
    pub const fn all(&self) -> bool {
        self.all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_denies() {
        let config = IvmTransactionAllowConfig::default();
        assert!(!config.all);
        assert!(config.to.is_empty());
        assert!(config.sender.is_empty());
    }

    #[test]
    fn allow_all() {
        let config = IvmTransactionAllowConfig::allow_all();
        assert!(config.all);
        assert!(config.to.is_empty());
        assert!(config.sender.is_empty());

        // Any transaction should be allowed
        assert!(config.is_allowed(&Address::with_last_byte(1), Some(Address::with_last_byte(2))));
        assert!(config.is_allowed(&Address::with_last_byte(3), None));
    }

    #[test]
    fn deny_all() {
        let config = IvmTransactionAllowConfig::deny_all();
        assert!(!config.all);
        assert!(config.to.is_empty());
        assert!(config.sender.is_empty());

        // No transaction should be allowed
        assert!(!config.is_allowed(&Address::with_last_byte(1), Some(Address::with_last_byte(2))));
        assert!(!config.is_allowed(&Address::with_last_byte(3), None));
    }

    #[test]
    fn allowed_sender() {
        let mut config = IvmTransactionAllowConfig::deny_all();
        let allowed_sender = Address::with_last_byte(1);
        config.add_sender(allowed_sender);

        // Test is_allowed with allowed sender
        assert!(config.is_allowed(&allowed_sender, Some(Address::with_last_byte(2))));
        assert!(config.is_allowed(&allowed_sender, None));

        // Test is_allowed with unauthorized sender
        assert!(!config.is_allowed(&Address::with_last_byte(3), Some(Address::with_last_byte(2))));
    }

    #[test]
    fn allowed_recipient() {
        let mut config = IvmTransactionAllowConfig::deny_all();
        let allowed_recipient = Address::with_last_byte(1);
        config.add_to(allowed_recipient);

        // Test with allowed recipient
        assert!(config.is_allowed(&Address::with_last_byte(2), Some(allowed_recipient)));

        // Test with unauthorized recipient
        assert!(!config.is_allowed(&Address::with_last_byte(2), Some(Address::with_last_byte(3))));

        // Test with no recipient
        assert!(!config.is_allowed(&Address::with_last_byte(2), None));
    }

    #[test]
    fn multiple_allowed_addresses() {
        let mut config = IvmTransactionAllowConfig::deny_all();

        // Set up multiple allowed senders and recipients
        let sender1 = Address::with_last_byte(1);
        let sender2 = Address::with_last_byte(2);
        let recipient1 = Address::with_last_byte(3);
        let recipient2 = Address::with_last_byte(4);

        config.add_sender(sender1);
        config.add_sender(sender2);

        config.add_to(recipient1);
        config.add_to(recipient2);

        // Test various combinations
        assert!(config.is_allowed(&sender1, Some(recipient1)));
        assert!(config.is_allowed(&sender1, Some(recipient2)));
        assert!(config.is_allowed(&sender2, Some(recipient1)));
        assert!(config.is_allowed(&sender2, Some(recipient2)));

        // Test unauthorized combinations
        assert!(!config.is_allowed(&Address::with_last_byte(5), Some(Address::with_last_byte(6))));
        assert!(!config.is_allowed(&Address::with_last_byte(5), None));
    }
}
