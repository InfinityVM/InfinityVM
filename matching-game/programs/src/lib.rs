//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy::{
        primitives::keccak256,
        sol_types::{SolType, SolValue},
    };
    use matching_game_core::{
        api::{CancelNumberRequest, Match, Request, SubmitNumberRequest}, Matches,
    };

    use abi::{StatefulAppOnchainInput, StatefulAppResult};
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
    
    fn apply_requests(txn: &mut Transaction<impl Store<Value = Vec<u8>>>, requests: &[Request]) -> Vec<Match> {
        let mut matches = Vec::<Match>::with_capacity(requests.len());

        for r in requests {
            match r {
                Request::SubmitNumber(s) => {

                    let mut old_list = txn.entry(&hash(s.number)).unwrap();
                    match old_list {
                        Occupied(mut entry) => {
                            let mut old_list = deserialize_address_list(entry.get());
                            if old_list.is_empty() {
                                old_list.push(s.address);
                            } else {
                                let match_pair = Match { user1: old_list[0].into(), user2: s.address.into() };
                                matches.push(match_pair);

                                // remove the first element from the list
                                old_list.remove(0);
                            }
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

        matches.sort();
        matches
    }
    
    #[test]
    fn submit_number_cancel_number_execute() {
        let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), TrieRoot::Empty));
        let hasher = &mut DigestHasher::<Sha256>::default();
        let mut merkle_root0 = txn.calc_root_hash(hasher).unwrap();

        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let charlie = [55u8; 20];

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];
        let (merkle_root1, matching_game_out) = execute(requests1.clone(), trie_db.clone(), merkle_root0);
        let merkle_root_1_option: Option<[u8; 32]> = merkle_root1.into();
        let merkle_root_1_bytes = merkle_root_1_option.unwrap();
        assert_eq!(*matching_game_out.output_state_root, merkle_root_1_bytes);
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert!(matches.is_empty());

        let requests2 = vec![
            // Sell 100 base for 4*100 quote
            Request::SubmitNumber(SubmitNumberRequest { address: charlie, number: 69 }),
            Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 }),
        ];
        let (merkle_root2, matching_game_out) = execute(requests2.clone(), trie_db.clone(), merkle_root1);
        let merkle_root_2_option: Option<[u8; 32]> = merkle_root2.into();
        let merkle_root_2_bytes = merkle_root_2_option.unwrap();
        assert_eq!(*matching_game_out.output_state_root, merkle_root_2_bytes);
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert_eq!(matches, vec![Match { user1: bob.into(), user2: charlie.into() }]);
    }

    fn execute(requests: Vec<Request>, trie_db: Rc<MemoryDb<Vec<u8>>>, pre_txn_merkle_root: TrieRoot<NodeHash>) -> (TrieRoot<NodeHash>, StatefulAppResult) {
        let requests_borsh = borsh::to_vec(&requests).expect("borsh works. qed.");

        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), pre_txn_merkle_root));
        let matches = apply_requests(&mut txn, &requests);
        let hasher = &mut DigestHasher::<Sha256>::default();
        let output_merkle_root = txn.commit(hasher).unwrap();

        let snapshot = txn.build_initial_snapshot();
        let snapshot_serialized = serde_json::to_vec(&snapshot).expect("serde works. qed.");
        
        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&snapshot_serialized);

        let merkle_root_thirty_two: Option<[u8; 32]> = pre_txn_merkle_root.into();
        let onchain_input_state_root = if merkle_root_thirty_two.is_none() {
            Default::default()
        } else {
            merkle_root_thirty_two.unwrap()
        };
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

        (output_merkle_root, <StatefulAppResult as SolValue>::abi_decode(&out_bytes, true).unwrap())
    }
}
