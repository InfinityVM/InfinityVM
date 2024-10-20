//! Types for the program and contract APIs. This also includes the internal
//! type used for accounting, [Diff].

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
    /// included in StatefulProgramResult.
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
