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
    use kairos_trie::{
        stored::{memory_db::MemoryDb, merkle::SnapshotBuilder, Store},
        DigestHasher, KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
        Entry::{Occupied, Vacant, VacantEmptyTrie},
    };
    use std::rc::Rc;
    use sha2::Sha256;

    fn hash(key: u64) -> KeyHash {
        let hasher = &mut DigestHasher::<Sha256>::default();
        key.portable_hash(hasher);
        KeyHash::from_bytes(&hasher.finalize_reset())
    }
    
    pub fn serialize_address_list(addresses: &Vec<[u8; 20]>) -> Vec<u8> {
        borsh::to_vec(addresses).expect("borsh works. qed.")
    }
    
    pub fn deserialize_address_list(data: &[u8]) -> Vec<[u8; 20]> {
        borsh::from_slice(data).expect("borsh works. qed.")
    }
    
    fn apply_requests(txn: &mut Transaction<impl Store<Value = Vec<u8>>>, requests: &[Request]) {
        for r in requests {
            match r {
                Request::SubmitNumber(s) => {

                    let mut old_list = txn.entry(&hash(s.number)).unwrap();
                    match old_list {
                        Occupied(mut entry) => {
                            let mut old_list = deserialize_address_list(entry.get());
                            old_list.push(s.address);
                            let _ = entry.insert(serialize_address_list(&old_list));
                        }
                        Vacant(_) => {
                            let _ = txn.insert(&hash(s.number), serialize_address_list(&vec![s.address]));
                        }
                        VacantEmptyTrie(_) => {
                            let _ = txn.insert(&hash(s.number), serialize_address_list(&vec![s.address]));
                        }
                    }
                }
                Request::CancelNumber(c) => {
                    let mut old_list = txn.entry(&hash(c.number)).unwrap();
                    match old_list {
                        Occupied(mut entry) => {
                            let mut old_list = deserialize_address_list(entry.get());
                            old_list.remove(old_list.iter().position(|&x| x == c.address).unwrap());
                            let _ = entry.insert(serialize_address_list(&old_list));
                        }
                        Vacant(_) => {
                            // do nothing
                        }
                        VacantEmptyTrie(_) => {
                            // do nothing
                        }
                    }
                }
            }
        }
    }
    
    #[test]
    fn submit_number_cancel_number_execute() {
        let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), TrieRoot::Empty));
        let hasher = &mut DigestHasher::<Sha256>::default();
    
        let mut pre_txn_merkle_root = txn.calc_root_hash(hasher).unwrap();


        // let matching_game_state0 = MatchingGameState::default();
        // let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
        // let mut initial_root = Default::default();
        // let merkle_trie0 = RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();
        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let charlie = [55u8; 20];

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];
        // let (matching_game_state1, merkle_trie1) =
        //     next_state(requests1.clone(), matching_game_state0.clone(), merkle_trie0);
        let matching_game_out = execute(requests1.clone(), trie_db.clone(), pre_txn_merkle_root);
        println!("NARULA matching_game_out.output_state_root: {:?}", matching_game_out.output_state_root.as_slice());
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

    fn execute(requests: Vec<Request>, trie_db: Rc<MemoryDb<Vec<u8>>>, pre_txn_merkle_root: TrieRoot<NodeHash>) -> StatefulAppResult {
        let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");

        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), pre_txn_merkle_root));

        apply_requests(&mut txn, &requests);

        let hasher = &mut DigestHasher::<Sha256>::default();
        let output_merkle_root = txn.commit(hasher).unwrap();
        let output_merkle_root_thirty_two: Option<[u8; 32]> = output_merkle_root.into();
        let output_merkle_root_result = if output_merkle_root_thirty_two.is_none() {
            Default::default()
        } else {
            output_merkle_root_thirty_two.unwrap()
        };
        println!("NARULA output_merkle_root_result in prover: {:?}", output_merkle_root_result);

        let snapshot = txn.build_initial_snapshot();

        let merkle_root_thirty_two: Option<[u8; 32]> = pre_txn_merkle_root.into();
        let onchain_input_state_root = if merkle_root_thirty_two.is_none() {
            Default::default()
        } else {
            merkle_root_thirty_two.unwrap()
        };

        let snapshot_serialized = serde_json::to_vec(&snapshot).expect("serde works. qed.");
        
        // let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
        // let mut initial_root = Default::default();

        // {
        //     let mut merkle_trie =
        //         RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();

        //     // for (number, addresses) in &init_state.number_to_addresses {
        //     //     merkle_trie
        //     //         .insert(number.to_le_bytes().as_slice(), &hash_addresses(addresses))
        //     //         .unwrap();
        //     // }
        //     init_state.merkle_root = *merkle_trie.root();
        // }

        // let mut trie_nodes = TrieNodes { numbers: vec![], addresses: vec![], proof: vec![] };
        // for txn in txns {
        //     let number = match txn {
        //         Request::SubmitNumber(s) => s.number,
        //         Request::CancelNumber(c) => c.number,
        //     };

        //     trie_nodes.numbers.push(number);
        //     // trie_nodes
        //     //     .addresses
        //     //     .push(init_state.number_to_addresses.get(&number).unwrap_or(&Vec::new()).clone());
        // }

        // // First, create a vector to hold the byte representations
        // let number_bytes: Vec<Vec<u8>> =
        //     trie_nodes.numbers.iter().map(|&n| n.to_le_bytes().to_vec()).collect();

        // // Then, create a vector of slices referencing these bytes
        // let number_slices: Vec<&[u8]> = number_bytes.iter().map(|v| v.as_slice()).collect();

        // // Now generate the proof
        // trie_nodes.proof = generate_proof::<
        //     MemoryDB<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>,
        //     ExtensionLayout,
        //     &Vec<&[u8]>,
        //     &[u8],
        // >(&memory_db, &init_state.merkle_root, &number_slices)
        // .unwrap();

        // let trie_nodes_borsh = borsh::to_vec(&trie_nodes).expect("borsh works. qed.");

        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&snapshot_serialized);
        // combined_offchain_input.extend_from_slice(&trie_nodes_borsh);

        let onchain_input = StatefulAppOnchainInput {
            input_state_root: onchain_input_state_root.into(),
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
