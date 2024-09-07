//! Types for the program and contract APIs. This also includes the internal
//! type used for accounting, [Diff].

use std::collections::HashMap;

use alloy::primitives::{Address, I256, U256};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

alloy::sol! {
    /// Balance delta for deposit.
     #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct DepositDelta {
        /// Target account.
        address account;
        /// Delta of free base asset.
        uint256 base;
        /// Delta of free quote asset.
        uint256 quote;
    }

    /// Balance delta for withdraw.
     #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct WithdrawDelta {
        /// Target account.
        address account;
        /// Delta of free base asset.
        uint256 base;
        /// Delta of free quote asset.
        uint256 quote;
    }

    /// Balance delta for fill, create, and cancel.
     #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct OrderDelta {
        /// Target account.
        address account;
        /// Delta of free base asset.
        int256 free_base;
        /// Delta of locked base asset.
        int256 locked_base;
        /// Delta of free quote asset.
        int256 free_quote;
        /// Delta of locked quote asset.
        int256 locked_quote;
    }

    /// Result deltas for clob. The ABI-encoded form of this is
    /// included in ClobProgramOutput.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct ClobResultDeltas {
        /// Deposit balance deltas.
        DepositDelta[] deposit_deltas;
        /// Order balance deltas.
        OrderDelta[] order_deltas;
        /// Withdraw balance deltas.
        WithdrawDelta[] withdraw_deltas;
    }
}

/// A state diff to balances. Uses rust native types and allows us to
/// derive for DB storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Diff {
    /// Funds are withdrawn from free balance.
    /// Burn base/quote amounts from free.
    Withdraw {
        /// User address
        user: [u8; 20],
        /// Base asset.
        base: u64,
        /// Quote asset.
        quote: u64,
    },
    /// Funds are exchanged between buyer and seller.
    /// Buyer credited base and debited quote.
    /// Seller credited quote and debited base.
    Fill {
        /// Buyer address.
        buyer: [u8; 20],
        /// Seller address.
        seller: [u8; 20],
        /// Base asset.
        base: u64,
        /// Quote asset.
        quote: u64,
    },
    /// Funds are locked to create an order.
    /// Credit base/quote amounts to locked and debit from free.
    Create {
        /// User address
        user: [u8; 20],
        /// Base asset.
        base: u64,
        /// Quote asset.
        quote: u64,
    },
    /// Funds are moved from locked to free.
    /// Credit base/quote amounts to free and debit from locked.
    Cancel {
        /// User address
        user: [u8; 20],
        /// Base asset.
        base: u64,
        /// Quote asset.
        quote: u64,
    },
    /// Funds are moved into the users free balance.
    /// Mint base/quote amounts into free.
    Deposit {
        /// User address
        user: [u8; 20],
        /// Base asset.
        base: u64,
        /// Quote asset.
        quote: u64,
    },
    /// An error occurred and no state was changed
    Noop,
}

impl Diff {
    /// Return a withdraw dif.
    pub const fn withdraw(user: [u8; 20], base: u64, quote: u64) -> Self {
        Self::Withdraw { user, base, quote }
    }
    /// Return a fill dif.
    pub const fn fill(buyer: [u8; 20], seller: [u8; 20], base: u64, quote: u64) -> Self {
        Self::Fill { buyer, seller, base, quote }
    }
    /// Return a create dif.
    pub const fn create(user: [u8; 20], base: u64, quote: u64) -> Self {
        Self::Create { user, base, quote }
    }
    /// Return a cancel dif.
    pub const fn cancel(user: [u8; 20], base: u64, quote: u64) -> Self {
        Self::Cancel { user, base, quote }
    }
    /// Return a cancel dif.
    pub const fn deposit(user: [u8; 20], base: u64, quote: u64) -> Self {
        Self::Deposit { user, base, quote }
    }

    /// Apply a diff to maps of delta types.
    pub(crate) fn apply(
        &self,
        withdraws: &mut HashMap<[u8; 20], WithdrawDelta>,
        deposits: &mut HashMap<[u8; 20], DepositDelta>,
        orders: &mut HashMap<[u8; 20], OrderDelta>,
    ) {
        match self {
            Self::Withdraw { user, base, quote } => {
                let delta = withdraws.entry(*user).or_insert(WithdrawDelta {
                    account: Address::from(user),
                    ..Default::default()
                });
                delta.base += U256::try_from(*base).expect("works");
                delta.quote += U256::try_from(*quote).expect("works");
            }
            Self::Deposit { user, base, quote } => {
                let delta = deposits
                    .entry(*user)
                    .or_insert(DepositDelta { account: Address::from(user), ..Default::default() });
                delta.base += U256::try_from(*base).expect("works");
                delta.quote += U256::try_from(*quote).expect("works");
            }
            Self::Fill { buyer, seller, base, quote } => {
                let buyer_delta = orders
                    .entry(*buyer)
                    .or_insert(OrderDelta { account: Address::from(buyer), ..Default::default() });
                buyer_delta.locked_quote -= I256::try_from(*quote).expect("works");
                buyer_delta.free_base += I256::try_from(*base).expect("works");

                let seller_delta = orders
                    .entry(*seller)
                    .or_insert(OrderDelta { account: Address::from(seller), ..Default::default() });
                seller_delta.locked_base -= I256::try_from(*base).expect("works");
                seller_delta.free_quote += I256::try_from(*quote).expect("works");
            }
            Self::Cancel { user, base, quote } => {
                let delta = orders
                    .entry(*user)
                    .or_insert(OrderDelta { account: Address::from(user), ..Default::default() });
                let base = I256::try_from(*base).expect("works");
                let quote = I256::try_from(*quote).expect("works");

                delta.locked_base -= base;
                delta.free_base += base;
                delta.locked_quote -= quote;
                delta.free_quote += quote;
            }
            Self::Create { user, base, quote } => {
                let delta = orders
                    .entry(*user)
                    .or_insert(OrderDelta { account: Address::from(user), ..Default::default() });
                let base = I256::try_from(*base).expect("works");
                let quote = I256::try_from(*quote).expect("works");

                delta.locked_base += base;
                delta.free_base -= base;
                delta.locked_quote += quote;
                delta.free_quote -= quote;
            }
            Self::Noop => {}
        }
    }
}
