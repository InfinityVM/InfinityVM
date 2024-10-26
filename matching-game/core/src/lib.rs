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
use trie_db::{Trie, TrieDBMutBuilder, TrieMut, TrieDBMut};
use memory_db::{MemoryDB, HashKey};
use keccak_hasher::KeccakHasher;
use reference_trie::{ExtensionLayout, RefTrieDBMut, RefTrieDBMutBuilder};

/// Matching game server API types.
pub mod api;

use crate::api::Match;

/// The state of the universe for the matching game.
#[derive(
Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct MatchingGameState {
    /// Map of number to list of addresses that submitted it.
    pub number_to_addresses: HashMap<u64, Vec<[u8; 20]>>,
    /// The merkle root of the state.
    pub merkle_root: [u8; 32],
}

impl MatchingGameState {
    /// Creates a new `MatchingGameState` with default values.
    pub fn new() -> Self {
        let merkle_root = Default::default();

        Self {
            number_to_addresses: HashMap::new(),
            merkle_root,
        }
    }

    /// Get the map of number to list of addresses that submitted it.
    pub fn get_number_to_addresses(&self) -> &HashMap<u64, Vec<[u8; 20]>> {
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
        match_pair = Some(MatchPair { user1: other_address, user2: address });
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

/// A tick will execute a single request against the matching game state.
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
        output_state_root: state.merkle_root.into(),
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

#[cfg(test)]
mod tests {
    use crate::{
        api::{
            CancelNumberRequest, CancelNumberResponse, MatchPair, Request, Response,
            SubmitNumberRequest, SubmitNumberResponse,
        },
        tick, MatchingGameState,
    };

    #[test]
    fn submit_number_cancel_number() {
        // let matching_game_state = MatchingGameState::default();
        // let bob = [69u8; 20];
        // let alice = [42u8; 20];

        // let alice_submit =
        //     Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 });
        // let (resp, matching_game_state) = tick(alice_submit, matching_game_state);
        // assert_eq!(
        //     Response::SubmitNumber(SubmitNumberResponse { success: true, match_pair: None }),
        //     resp
        // );
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&42).unwrap(), vec![alice]);

        // let bob_submit = Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 });
        // let (resp, matching_game_state) = tick(bob_submit, matching_game_state);
        // assert_eq!(
        //     Response::SubmitNumber(SubmitNumberResponse { success: true, match_pair: None }),
        //     resp
        // );
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&42).unwrap(), vec![alice]);
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&69).unwrap(), vec![bob]);

        // let alice_cancel =
        //     Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 });
        // let (resp, matching_game_state) = tick(alice_cancel, matching_game_state);
        // assert_eq!(Response::CancelNumber(CancelNumberResponse { success: true }), resp);
        // assert!(matching_game_state.get_number_to_addresses().get(&42).unwrap().is_empty());
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&69).unwrap(), vec![bob]);

        // let alice_second_submit =
        //     Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 69 });
        // let (resp, matching_game_state) = tick(alice_second_submit, matching_game_state);
        // assert_eq!(
        //     Response::SubmitNumber(SubmitNumberResponse {
        //         success: true,
        //         match_pair: Some(MatchPair { user1: bob, user2: alice })
        //     }),
        //     resp
        // );
        // assert!(matching_game_state.get_number_to_addresses().get(&42).unwrap().is_empty());
        // assert!(matching_game_state.get_number_to_addresses().get(&69).unwrap().is_empty());
    }
}
