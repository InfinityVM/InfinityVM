//! Core logic and types of the matching game.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture.

use std::collections::HashMap;

use alloy::{
    primitives::{utils::keccak256, FixedBytes},
    sol,
    sol_types::SolType,
};

use abi::StatefulAppResult;
use api::{
    CancelNumberRequest, CancelNumberResponse, MatchPair, Request, Response, SubmitNumberRequest,
    SubmitNumberResponse,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

pub mod api;

use crate::api::Match;

/// The state of the universe for the matching game.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct MatchingGameState {
    /// Map of number to list of addresses that submitted it.
    pub number_to_addresses: HashMap<u64, Vec<[u8; 20]>>,
}

impl MatchingGameState {
    /// Get the map of number to list of addresses that submitted it.
    pub const fn get_number_to_addresses(&self) -> &HashMap<u64, Vec<[u8; 20]>> {
        &self.number_to_addresses
    }
}

/// Add a submission with a user's favorite number.
pub fn submit_number(
    req: SubmitNumberRequest,
    mut state: MatchingGameState,
) -> (SubmitNumberResponse, MatchingGameState) {
    let number = req.number;
    let address = req.address;
    let mut match_pair = None;

    // check if the number is already submitted by this address
    if state
        .number_to_addresses
        .get(&number)
        .map_or(false, |addresses| addresses.contains(&address))
    {
        return (SubmitNumberResponse { success: false, match_pair }, state);
    }

    if let Some(addresses) = state.number_to_addresses.get_mut(&number) {
        // if the number is already submitted by other addresses, remove the first address in that
        // list and create a match pair
        let other_address = addresses.remove(0);
        match_pair = Some(MatchPair { user1: other_address.into(), user2: address.into() });
    } else {
        // if not, add the submission
        state.number_to_addresses.entry(number).or_default().push(address);
    }

    (SubmitNumberResponse { success: true, match_pair }, state)
}

/// Cancel a submission of a number.
pub fn cancel_number(
    req: CancelNumberRequest,
    mut state: MatchingGameState,
) -> (CancelNumberResponse, MatchingGameState) {
    let number = req.number;
    let address = req.address;

    if !state.number_to_addresses.get_mut(&number).map_or(false, |addresses| {
        addresses.iter().position(|&a| a == address).map(|i| addresses.remove(i)).is_some()
    }) {
        return (CancelNumberResponse { success: false }, state);
    }

    (CancelNumberResponse { success: true }, state)
}

/// A tick will execute a single request against the CLOB state.
pub fn tick(request: Request, state: MatchingGameState) -> (Response, MatchingGameState) {
    match request {
        Request::SubmitNumber(req) => {
            let (resp, state) = submit_number(req, state);
            (Response::SubmitNumber(resp), state)
        }
        Request::CancelNumber(req) => {
            let (resp, state) = cancel_number(req, state);
            (Response::CancelNumber(resp), state)
        }
    }
}

/// Array of matches. Result from matching game program.
pub type Matches = sol! {
    Match[]
};

/// State transition function used in the ZKVM. It only outputs balance changes, which are abi
/// encoded for contract consumption.
pub fn zkvm_stf(requests: Vec<Request>, mut state: MatchingGameState) -> StatefulAppResult {
    let mut matches = Vec::<Match>::with_capacity(requests.len());

    for req in requests {
        let (resp, next_state) = tick(req, state);
        if let Response::SubmitNumber(resp) = resp {
            if let Some(match_pair) = resp.match_pair {
                let match_struct =
                    Match { user1: match_pair.user1.into(), user2: match_pair.user2.into() };
                matches.push(match_struct);
            }
        }

        state = next_state
    }

    matches.sort();

    StatefulAppResult {
        output_state_root: state.borsh_keccak256(),
        result: Matches::abi_encode(&matches).into(),
    }
}

/// Trait for the keccak256 hash of a borsh serialized type
pub trait BorshKeccak256 {
    /// The keccak256 hash of a borsh serialized type
    fn borsh_keccak256(&self) -> FixedBytes<32>;
}

impl<T: BorshSerialize> BorshKeccak256 for T {
    fn borsh_keccak256(&self) -> FixedBytes<32> {
        let borsh = borsh::to_vec(&self).expect("T is serializable");
        keccak256(&borsh)
    }
}
