//! ZKVM program for running the matching game.

use alloy::sol_types::SolType;
use abi::{StatefulAppOnchainInput, StatefulAppResult};
use risc0_zkvm::guest::env;
use trie_db::proof::verify_proof;
use matching_game_core::{api::Request, MatchingGameState, zkvm_stf, TrieNodes, hash_addresses};
use reference_trie::ExtensionLayout;

fn main() {
    let onchain_input_len: u32 = env::read();
    let mut onchain_input_buf = vec![0u8; onchain_input_len as usize];
    env::read_slice(&mut onchain_input_buf);

    let stateful_app_input = StatefulAppOnchainInput::abi_decode(&onchain_input_buf, false).unwrap();
    let input_merkle_root = stateful_app_input.input_state_root;

    let offchain_input_len: u32 = env::read();
    let mut offchain_input_buf = vec![0u8; offchain_input_len as usize];
    env::read_slice(&mut offchain_input_buf);

    // We combined the requests and trie nodes in offchain input, so we need to split them here.
    let requests_len = u32::from_le_bytes(offchain_input_buf[..4].try_into().unwrap()) as usize;
    let requests_borsh = offchain_input_buf[4..4 + requests_len].to_vec();
    let trie_nodes_borsh = offchain_input_buf[4 + requests_len..].to_vec();

    let requests: Vec<Request> = borsh::from_slice(&requests_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");
    let trie_nodes: TrieNodes = borsh::from_slice(&trie_nodes_borsh)
        .expect("TODO: https://github.com/InfinityVM/InfinityVM/issues/296");

    let mut items = Vec::new();
    for i in 0..trie_nodes.numbers.len() {
        let number_bytes = trie_nodes.numbers[i].to_le_bytes().to_vec();
        if trie_nodes.addresses[i].is_empty() {
            items.push((number_bytes, None));
        } else {
            let addresses_hash = hash_addresses(trie_nodes.addresses[i].as_slice());
            items.push((number_bytes, Some(addresses_hash)));
        }
    }
    // verify the proof
    verify_proof::<ExtensionLayout, &Vec<(Vec<u8>, Option<[u8; 32]>)>, Vec<u8>, [u8; 32]>(&input_merkle_root, &trie_nodes.proof, &items).unwrap();

    let mut state = MatchingGameState::default();
    state.merkle_root = *input_merkle_root;
    for (number, addresses) in trie_nodes.numbers.iter().zip(trie_nodes.addresses.iter()) {
        if !addresses.is_empty() {
            state.number_to_addresses.insert(*number, addresses.clone());
        }
    }

    let matching_game_program_output = zkvm_stf(requests, state);
    let abi_encoded = StatefulAppResult::abi_encode(&matching_game_program_output);

    env::commit_slice(&abi_encoded);
}
