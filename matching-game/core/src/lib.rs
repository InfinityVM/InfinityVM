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
use keccak_hasher::KeccakHasher;
use memory_db::{HashKey, MemoryDB};
use reference_trie::{RefTrieDBMut, RefTrieDBMutBuilder};
use serde::{Deserialize, Serialize};
use trie_db::{Trie, TrieDBMut, TrieDBMutBuilder, TrieMut};

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
    /// The root of the state merkle trie.
    pub merkle_root: [u8; 32],
}

impl MatchingGameState {
    /// Get the map of number to list of addresses that submitted it.
    pub fn get_number_to_addresses(&self) -> &HashMap<u64, Vec<[u8; 20]>> {
        &self.number_to_addresses
    }

    /// Get the root of the state merkle trie.
    pub fn get_merkle_root(&self) -> &[u8; 32] {
        &self.merkle_root
    }
}

/// Add a submission with a user's favorite number.
pub fn submit_number<'a>(
    req: SubmitNumberRequest,
    mut state: MatchingGameState,
    mut merkle_trie: RefTrieDBMut<'a>,
) -> (SubmitNumberResponse, MatchingGameState, RefTrieDBMut<'a>) {
    let number = req.number;
    let address = req.address;
    let mut match_pair = None;

    // check if the number is already submitted by this address
    if state
        .number_to_addresses
        .get(&number)
        .map_or(false, |addresses| addresses.contains(&address))
    {
        return (SubmitNumberResponse { success: false, match_pair }, state, merkle_trie);
    }

    if let Some(addresses) = state.number_to_addresses.get_mut(&number) {
        // if the number is already submitted by other addresses, remove the first address in that
        // list and create a match pair
        let other_address = addresses.remove(0);
        match_pair = Some(MatchPair { user1: other_address, user2: address });

        // update the merkle trie
        merkle_trie.insert(number.to_le_bytes().as_slice(), &hash_addresses(&addresses)).unwrap();
        state.merkle_root = *merkle_trie.root();
    } else {
        // if not, add the submission
        state.number_to_addresses.entry(number).or_default().push(address);

        // update the merkle trie
        merkle_trie.insert(number.to_le_bytes().as_slice(), &hash_addresses(&[address])).unwrap();
        state.merkle_root = *merkle_trie.root();
    }

    (SubmitNumberResponse { success: true, match_pair }, state, merkle_trie)
}

/// Cancel a submission of a number.
pub fn cancel_number<'a>(
    req: CancelNumberRequest,
    mut state: MatchingGameState,
    mut merkle_trie: RefTrieDBMut<'a>,
) -> (CancelNumberResponse, MatchingGameState, RefTrieDBMut<'a>) {
    let number = req.number;
    let address = req.address;

    if !state.number_to_addresses.get_mut(&number).map_or(false, |addresses| {
        addresses
            .iter()
            .position(|&a| a == address)
            .map(|i| {
                let remove_result = addresses.remove(i);

                // update the merkle trie
                merkle_trie
                    .insert(number.to_le_bytes().as_slice(), &hash_addresses(&addresses))
                    .unwrap();
                state.merkle_root = *merkle_trie.root();

                remove_result
            })
            .is_some()
    }) {
        return (CancelNumberResponse { success: false }, state, merkle_trie);
    }

    (CancelNumberResponse { success: true }, state, merkle_trie)
}

/// A tick will execute a single request against the matching game state.
pub fn tick<'a>(
    request: Request,
    state: MatchingGameState,
    merkle_trie: RefTrieDBMut<'a>,
) -> (Response, MatchingGameState, RefTrieDBMut<'a>) {
    match request {
        Request::SubmitNumber(req) => {
            let (resp, state, merkle_trie) = submit_number(req, state, merkle_trie);
            (Response::SubmitNumber(resp), state, merkle_trie)
        }
        Request::CancelNumber(req) => {
            let (resp, state, merkle_trie) = cancel_number(req, state, merkle_trie);
            (Response::CancelNumber(resp), state, merkle_trie)
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

    let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
    let mut initial_root = Default::default();
    let mut merkle_trie = RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();

    // Go through the number_to_addresses and insert into the merkle trie
    for (number, addresses) in &state.number_to_addresses {
        merkle_trie.insert(number.to_le_bytes().as_slice(), &hash_addresses(addresses)).unwrap();
    }
    state.merkle_root = *merkle_trie.root();

    for req in requests {
        let (resp, next_state, next_merkle_trie) = tick(req, state, merkle_trie);
        if let Response::SubmitNumber(resp) = resp {
            if let Some(match_pair) = resp.match_pair {
                let match_struct =
                    Match { user1: match_pair.user1.into(), user2: match_pair.user2.into() };
                matches.push(match_struct);
            }
        }

        state = next_state;
        merkle_trie = next_merkle_trie;
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

/// Hasher function for list of addresses
pub fn hash_addresses(addresses: &[[u8; 20]]) -> [u8; 32] {
    let concatenated = addresses.concat();
    *keccak256(&concatenated)
}

/// Used to pass each trie node as input to the ZKVM.
#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct TrieNodes {
    /// Favorite numbers of the user.
    pub numbers: Vec<u64>,
    /// Lists of addresses that submitted the numbers.
    pub addresses: Vec<Vec<[u8; 20]>>,
    /// Proof of the inclusion of all the (number, addresses) pairs in the trie.
    pub proof: Vec<Vec<u8>>,
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
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&42).unwrap(),
        // vec![alice]);

        // let bob_submit = Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 });
        // let (resp, matching_game_state) = tick(bob_submit, matching_game_state);
        // assert_eq!(
        //     Response::SubmitNumber(SubmitNumberResponse { success: true, match_pair: None }),
        //     resp
        // );
        // assert_eq!(*matching_game_state.get_number_to_addresses().get(&42).unwrap(),
        // vec![alice]); assert_eq!(*matching_game_state.get_number_to_addresses().get(&69).
        // unwrap(), vec![bob]);

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
