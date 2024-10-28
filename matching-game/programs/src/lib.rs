//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy::{
        primitives::keccak256,
        sol_types::{SolType, SolValue},
    };
    use matching_game_core::{
        api::{CancelNumberRequest, Match, Request, SubmitNumberRequest},
        hash_addresses, BorshKeccak256, Matches, MatchingGameState, TrieNodes,
    };
    use matching_game_test_utils::next_state;

    use abi::{StatefulAppOnchainInput, StatefulAppResult};
    use keccak_hasher::KeccakHasher;
    use memory_db::{HashKey, MemoryDB};
    use reference_trie::{ExtensionLayout, RefTrieDBMutBuilder};
    use trie_db::{proof::generate_proof, TrieMut};
    use zkvm::Zkvm;

    #[test]
    fn submit_number_cancel_number_execute() {
        // let matching_game_state0 = MatchingGameState::default();
        // let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
        // let mut initial_root = Default::default();
        // let merkle_trie0 = RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();
        // let bob = [69u8; 20];
        // let alice = [42u8; 20];
        // let charlie = [55u8; 20];

        // let requests1 = vec![
        //     Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
        //     Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        // ];
        // let (matching_game_state1, merkle_trie1) =
        //     next_state(requests1.clone(), matching_game_state0.clone(), merkle_trie0);
        // let matching_game_out = execute(requests1.clone(), matching_game_state0.clone());
        // assert_eq!(*matching_game_out.output_state_root, matching_game_state1.merkle_root);
        // let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        // assert!(matches.is_empty());

        // let requests2 = vec![
        //     // Sell 100 base for 4*100 quote
        //     Request::SubmitNumber(SubmitNumberRequest { address: charlie, number: 69 }),
        //     Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 }),
        // ];
        // let (matching_game_state2, merkle_trie2) =
        //     next_state(requests2.clone(), matching_game_state1.clone(), merkle_trie1);
        // let matching_game_out = execute(requests2.clone(), matching_game_state1.clone());
        // assert_eq!(matching_game_out.output_state_root, matching_game_state2.merkle_root);
        // let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        // assert_eq!(matches, vec![Match { user1: bob.into(), user2: charlie.into() }]);
    }

    fn execute(txns: Vec<Request>, mut init_state: MatchingGameState) -> StatefulAppResult {
        let requests_borsh = borsh::to_vec(&txns).expect("borsh works. qed.");

        let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
        let mut initial_root = Default::default();

        {
            let mut merkle_trie =
                RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();

            // for (number, addresses) in &init_state.number_to_addresses {
            //     merkle_trie
            //         .insert(number.to_le_bytes().as_slice(), &hash_addresses(addresses))
            //         .unwrap();
            // }
            init_state.merkle_root = *merkle_trie.root();
        }

        let mut trie_nodes = TrieNodes { numbers: vec![], addresses: vec![], proof: vec![] };
        for txn in txns {
            let number = match txn {
                Request::SubmitNumber(s) => s.number,
                Request::CancelNumber(c) => c.number,
            };

            trie_nodes.numbers.push(number);
            // trie_nodes
            //     .addresses
            //     .push(init_state.number_to_addresses.get(&number).unwrap_or(&Vec::new()).clone());
        }

        // First, create a vector to hold the byte representations
        let number_bytes: Vec<Vec<u8>> =
            trie_nodes.numbers.iter().map(|&n| n.to_le_bytes().to_vec()).collect();

        // Then, create a vector of slices referencing these bytes
        let number_slices: Vec<&[u8]> = number_bytes.iter().map(|v| v.as_slice()).collect();

        // Now generate the proof
        trie_nodes.proof = generate_proof::<
            MemoryDB<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>,
            ExtensionLayout,
            &Vec<&[u8]>,
            &[u8],
        >(&memory_db, &init_state.merkle_root, &number_slices)
        .unwrap();

        let trie_nodes_borsh = borsh::to_vec(&trie_nodes).expect("borsh works. qed.");

        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&trie_nodes_borsh);

        let onchain_input = StatefulAppOnchainInput {
            input_state_root: init_state.merkle_root.into(),
            onchain_input: [0].into(),
        };

        let out_bytes = zkvm::Risc0 {}
            .execute(
                super::MATCHING_GAME_ELF,
                <StatefulAppOnchainInput as SolValue>::abi_encode(&onchain_input).as_slice(),
                &combined_offchain_input,
                32 * 1000 * 1000,
            )
            .unwrap();

        <StatefulAppResult as SolValue>::abi_decode(&out_bytes, true).unwrap()
    }
}
