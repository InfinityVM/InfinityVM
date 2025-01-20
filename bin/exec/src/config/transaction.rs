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

    /// Set allowed `to` addresses.
    pub fn set_to(&mut self, to: HashSet<Address>) {
        self.to = to;
    }

    /// Add a `to` address.
    pub fn add_to(&mut self, to: Address) {
        self.to.insert(to);
    }

    /// Get `to` addresses.
    pub fn to(&self) -> HashSet<Address> {
        self.to.clone()
    }

    /// Set allowed `sender` addresses.
    pub fn set_sender(&mut self, sender: HashSet<Address>) {
        self.sender = sender;
    }

    /// Add a `sender` address.
    pub fn add_sender(&mut self, sender: Address) {
        self.sender.insert(sender);
    }

    /// Get allowed sender addresses.
    pub fn sender(&self) -> HashSet<Address> {
        self.sender.clone()
    }

    /// Allow all transactions
    pub fn set_all(&mut self, allow_all: bool) {
        self.all = allow_all;
    }

    /// Get if all transactions are allowed.
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
        let mut senders = HashSet::new();
        senders.insert(allowed_sender);
        config.set_sender(senders);

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
        let mut recipients = HashSet::new();
        recipients.insert(allowed_recipient);
        config.set_to(recipients);

        // Test with allowed recipient
        assert!(config.is_allowed(&Address::with_last_byte(2), Some(allowed_recipient)));

        // Test with unauthorized recipient
        assert!(!config.is_allowed(&Address::with_last_byte(2), Some(Address::with_last_byte(3))));

        // Test with no recipient
        assert!(!config.is_allowed(&Address::with_last_byte(2), None));
    }

    #[test]
    fn getters_and_setters() {
        let mut config = IvmTransactionAllowConfig::deny_all();

        // Test all flag
        assert!(!config.all());
        config.set_all(true);
        assert!(config.all());

        // Test sender set/get
        let sender = Address::with_last_byte(1);
        let mut senders = HashSet::new();
        senders.insert(sender);
        config.set_sender(senders.clone());
        assert_eq!(config.sender(), senders);

        // Test to set/get
        let recipient = Address::with_last_byte(2);
        let mut recipients = HashSet::new();
        recipients.insert(recipient);
        config.set_to(recipients.clone());
        assert_eq!(config.to(), recipients);
    }

    #[test]
    fn multiple_allowed_addresses() {
        let mut config = IvmTransactionAllowConfig::deny_all();

        // Set up multiple allowed senders and recipients
        let sender1 = Address::with_last_byte(1);
        let sender2 = Address::with_last_byte(2);
        let recipient1 = Address::with_last_byte(3);
        let recipient2 = Address::with_last_byte(4);

        let mut senders = HashSet::new();
        senders.insert(sender1);
        senders.insert(sender2);
        config.set_sender(senders);

        let mut recipients = HashSet::new();
        recipients.insert(recipient1);
        recipients.insert(recipient2);
        config.set_to(recipients);

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
