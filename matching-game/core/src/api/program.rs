//! Types for the program and contract APIs.

use std::collections::HashMap;

use alloy::{
    primitives::{Address, I256, U256},
    sol,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

alloy::sol! {
    /// Matching pair.
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    struct Match {
        address user1;
        address user2;
    }
}
