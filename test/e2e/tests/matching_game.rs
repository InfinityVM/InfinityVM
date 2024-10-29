use abi::{abi_encode_offchain_job_request, JobParams, StatefulAppOnchainInput, StatefulAppResult};
use alloy::{
    network::EthereumWallet,
    primitives::keccak256,
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use e2e::{Args, E2E};
use matching_game_contracts::matching_game_consumer::MatchingGameConsumer;
use matching_game_core::{api::{
        CancelNumberRequest, CancelNumberResponse, MatchPair, Request, SubmitNumberRequest,
        SubmitNumberResponse, Match,
    }, Matches};
use matching_game_programs::MATCHING_GAME_ELF;
use proto::{GetResultRequest, SubmitJobRequest, SubmitProgramRequest, VmType};
use risc0_binfmt::compute_image_id;
use zkvm_executor::service::OffchainResultWithMetadata;
use kairos_trie::{
    stored::{memory_db::MemoryDb, merkle::SnapshotBuilder, Store},
    DigestHasher, KeyHash, NodeHash, PortableHash, PortableHasher, Transaction, TrieRoot,
    Entry::{Occupied, Vacant, VacantEmptyTrie},
};
use std::rc::Rc;
use sha2::Sha256;

fn program_id() -> Vec<u8> {
    compute_image_id(MATCHING_GAME_ELF).unwrap().as_bytes().to_vec()
}


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

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn state_job_submission_matching_game_consumer() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let matching_game = args.matching_game_consumer.unwrap();
        let program_id = program_id();
        let matching_game_signer_wallet =
            EthereumWallet::from(matching_game.matching_game_signer.clone());
        
        let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        let alice: [u8; 20] = alice_key.address().into();
        let bob: [u8; 20] = bob_key.address().into();

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MATCHING_GAME_ELF.to_vec(),
            vm_type: VmType::Risc0.into(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(matching_game_signer_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];

        let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), TrieRoot::Empty));
        let hasher = &mut DigestHasher::<Sha256>::default();
        let mut merkle_root0 = txn.calc_root_hash(hasher).unwrap();

        let matches = apply_requests(&mut txn, &requests1);
        let hasher = &mut DigestHasher::<Sha256>::default();
        let merkle_root1 = txn.commit(hasher).unwrap();

        let requests2 =
            vec![Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 })];
        
        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), merkle_root1));
        let matches = apply_requests(&mut txn, &requests2);
        let hasher = &mut DigestHasher::<Sha256>::default();
        let merkle_root2 = txn.commit(hasher).unwrap();
        
        let requests3 =
            vec![Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 69 })];
        
        let mut txn =
            Transaction::from_snapshot_builder(SnapshotBuilder::new(trie_db.clone(), merkle_root2));
        let matches = apply_requests(&mut txn, &requests3);
        let hasher = &mut DigestHasher::<Sha256>::default();
        let merkle_root3 = txn.commit(hasher).unwrap();

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
        interval.tick().await; // First tick processes immediately
        let mut nonce = 2;

        for (requests, pre_txn_merkle_root, next_merkle_root) in [
            (requests1, merkle_root0, merkle_root1),
            (requests2, merkle_root1, merkle_root2),
            (requests3, merkle_root2, merkle_root3),
        ] {
            let requests_borsh = borsh::to_vec(&requests).unwrap();

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
            let offchain_input_hash = keccak256(&combined_offchain_input);

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
            let onchain_input_abi_encoded = StatefulAppOnchainInput::abi_encode(&onchain_input);

            let params = JobParams {
                nonce,
                max_cycles: 32 * 1000 * 1000,
                consumer_address: **matching_game.matching_game_consumer,
                onchain_input: onchain_input_abi_encoded.as_slice(),
                offchain_input_hash: offchain_input_hash.into(),
                program_id: &program_id,
            };
            let request = abi_encode_offchain_job_request(params.clone());
            let signature = matching_game
                .matching_game_signer
                .sign_message(&request)
                .await
                .unwrap()
                .as_bytes()
                .to_vec();
            let job_request =
                SubmitJobRequest { request, signature, offchain_input: combined_offchain_input };
            let submit_job_response =
                args.coprocessor_node.submit_job(job_request).await.unwrap().into_inner();

            // Wait for the job to be processed
            interval.tick().await;

            let job_id = submit_job_response.job_id;
            let offchain_result_with_metadata = args
                .coprocessor_node
                .get_result(GetResultRequest { job_id })
                .await
                .unwrap()
                .into_inner()
                .job_result
                .unwrap()
                .result_with_metadata;

            let raw_output = {
                let abi_decoded_output =
                    OffchainResultWithMetadata::abi_decode(&offchain_result_with_metadata, false)
                        .unwrap();
                abi_decoded_output.raw_output
            };

            {
                let matching_game_output =
                    StatefulAppResult::abi_decode(&raw_output, false).unwrap();
                let next_merkle_root_option: Option<[u8; 32]> = next_merkle_root.into();
                let next_merkle_root_bytes = next_merkle_root_option.unwrap();
                assert_eq!(*matching_game_output.output_state_root, next_merkle_root_bytes);
            }

            nonce += 1;
        }

        let consumer_contract =
            MatchingGameConsumer::new(matching_game.matching_game_consumer, &consumer_provider);
        let partner = consumer_contract.getPartner(alice.into()).call().await.unwrap()._0;
        assert_eq!(partner, bob);
        let partner = consumer_contract.getPartner(bob.into()).call().await.unwrap()._0;
        assert_eq!(partner, alice);

        // for (requests, mut init_state, next_state) in [
        //     (requests1, matching_game_state0.clone(), &matching_game_state1),
        //     (requests2, matching_game_state1.clone(), &matching_game_state2),
        //     (requests3, matching_game_state2.clone(), &matching_game_state3),
        // ] {
        //     let requests_borsh = borsh::to_vec(&requests).unwrap();

        //     let mut memory_db = MemoryDB::<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>::default();
        //     let mut initial_root = Default::default();

        //     {
        //         let mut merkle_trie =
        //             RefTrieDBMutBuilder::new(&mut memory_db, &mut initial_root).build();

        //         for (number, addresses) in &init_state.number_to_addresses {
        //             merkle_trie
        //                 .insert(number.to_le_bytes().as_slice(), &hash_addresses(addresses))
        //                 .unwrap();
        //         }
        //         init_state.merkle_root = *merkle_trie.root();
        //     }

    
        //     let mut trie_nodes = TrieNodes { numbers: vec![], addresses: vec![], proof: vec![] };
        //     for r in requests.clone() {
        //         let number = match r {
        //             Request::SubmitNumber(s) => s.number,
        //             Request::CancelNumber(c) => c.number,
        //         };
    
        //         trie_nodes.numbers.push(number);
        //         trie_nodes
        //             .addresses
        //             .push(init_state.number_to_addresses.get(&number).unwrap_or(&Vec::new()).clone());
        //     }
    
        //     // First, create a vector to hold the byte representations
        //     let number_bytes: Vec<Vec<u8>> =
        //         trie_nodes.numbers.iter().map(|&n| n.to_le_bytes().to_vec()).collect();
    
        //     // Then, create a vector of slices referencing these bytes
        //     let number_slices: Vec<&[u8]> = number_bytes.iter().map(|v| v.as_slice()).collect();
    
        //     // Now generate the proof
        //     trie_nodes.proof = generate_proof::<
        //         MemoryDB<KeccakHasher, HashKey<KeccakHasher>, Vec<u8>>,
        //         ExtensionLayout,
        //         &Vec<&[u8]>,
        //         &[u8],
        //     >(&memory_db, &init_state.merkle_root, &number_slices)
        //     .unwrap();
        //     let proof = trie_nodes.proof.clone();

        //     let mut items = Vec::new();
        //     for i in 0..trie_nodes.numbers.len() {
        //         let number_bytes = trie_nodes.numbers[i].to_le_bytes().to_vec();
        //         if trie_nodes.addresses[i].is_empty() {
        //             items.push((number_bytes, None));
        //         } else {
        //             let addresses_hash = hash_addresses(trie_nodes.addresses[i].as_slice());
        //             items.push((number_bytes, Some(addresses_hash)));
        //         }
        //     }
        
        //     reconstruct_root_from_proof(proof, items);
    
        //     let trie_nodes_borsh = borsh::to_vec(&trie_nodes).expect("borsh works. qed.");

        //     // print keccak256 of state_borsh
        //     let state_borsh_hash = keccak256(&trie_nodes_borsh);

        //     let mut state = MatchingGameState::default();
        //     state.merkle_root = init_state.merkle_root;
        //     for (number, addresses) in trie_nodes.numbers.iter().zip(trie_nodes.addresses.iter()) {
        //         if !addresses.is_empty() {
        //             state.number_to_addresses.insert(*number, addresses.clone());
        //         }
        //     }

        //     let state_borsh = borsh::to_vec(&state).expect("borsh works. qed.");
        //     let state_borsh_hash = keccak256(&state_borsh);
        

            // Combine requests_borsh and trie_nodes_borsh
        //     let mut combined_offchain_input = Vec::new();
        //     combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        //     combined_offchain_input.extend_from_slice(&requests_borsh);
        //     combined_offchain_input.extend_from_slice(&trie_nodes_borsh);

        //     let offchain_input_hash = keccak256(&combined_offchain_input);

        //     // run zkvm_stf
        //     let zkvm_stf_output = zkvm_stf(requests.clone(), init_state.clone());

        //     // run zkvm_stf on state_borsh
        //     let zkvm_stf_output_borsh = zkvm_stf(requests, state.clone());

        //     let onchain_input = StatefulAppOnchainInput {
        //         input_state_root: init_state.merkle_root.into(),
        //         onchain_input: (&[]).into(),
        //     };
        //     let onchain_input_abi_encoded = StatefulAppOnchainInput::abi_encode(&onchain_input);

        //     let params = JobParams {
        //         nonce,
        //         max_cycles: 32 * 1000 * 1000,
        //         consumer_address: **matching_game.matching_game_consumer,
        //         onchain_input: onchain_input_abi_encoded.as_slice(),
        //         offchain_input_hash: offchain_input_hash.into(),
        //         program_id: &program_id,
        //     };
        //     let request = abi_encode_offchain_job_request(params.clone());
        //     let signature = matching_game
        //         .matching_game_signer
        //         .sign_message(&request)
        //         .await
        //         .unwrap()
        //         .as_bytes()
        //         .to_vec();
        //     let job_request =
        //         SubmitJobRequest { request, signature, offchain_input: combined_offchain_input };
        //     let submit_job_response =
        //         args.coprocessor_node.submit_job(job_request).await.unwrap().into_inner();

        //     // Wait for the job to be processed
        //     interval.tick().await;

        //     let job_id = submit_job_response.job_id;
        //     let offchain_result_with_metadata = args
        //         .coprocessor_node
        //         .get_result(GetResultRequest { job_id })
        //         .await
        //         .unwrap()
        //         .into_inner()
        //         .job_result
        //         .unwrap()
        //         .result_with_metadata;

        //     let raw_output = {
        //         let abi_decoded_output =
        //             OffchainResultWithMetadata::abi_decode(&offchain_result_with_metadata, false)
        //                 .unwrap();
        //         abi_decoded_output.raw_output
        //     };

        //     {
        //         let matching_game_output =
        //             StatefulAppResult::abi_decode(&raw_output, false).unwrap();
        //         assert_eq!(*matching_game_output.output_state_root, next_state.merkle_root);
        //     }

        //     nonce += 1;
        // }

    }
    E2E::new().matching_game().run(test).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn matching_game_node_e2e() {
    async fn test(mut args: Args) {
        // let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(13));
        // interval.tick().await; // First tick processes immediately

        // let anvil = args.anvil;
        // let matching_game = args.matching_game_consumer.unwrap();
        // let program_id = program_id();
        // let matching_game_signer_wallet =
        //     EthereumWallet::from(matching_game.matching_game_signer.clone());
        // let matching_game_endpoint = args.matching_game_endpoint.unwrap();

        // let client = matching_game_client::Client::new(matching_game_endpoint);

        // // Setup ready to use on chain accounts for Alice & Bob
        // let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        // let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        // let alice: [u8; 20] = alice_key.address().into();
        // let bob: [u8; 20] = bob_key.address().into();

        // // Seed coprocessor-node with ELF
        // let submit_program_request = SubmitProgramRequest {
        //     program_elf: MATCHING_GAME_ELF.to_vec(),
        //     vm_type: VmType::Risc0.into(),
        // };
        // let submit_program_response = args
        //     .coprocessor_node
        //     .submit_program(submit_program_request)
        //     .await
        //     .unwrap()
        //     .into_inner();
        // assert_eq!(submit_program_response.program_id, program_id);

        // // Get chain state setup
        // let consumer_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(matching_game_signer_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let consumer_contract =
        //     MatchingGameConsumer::new(matching_game.matching_game_consumer, &consumer_provider);

        // let alice_submit_number = SubmitNumberRequest { address: alice, number: 42 };
        // let bob_submit_number = SubmitNumberRequest { address: bob, number: 69 };
        // let (r, i) = client.submit_number(alice_submit_number).await.unwrap();
        // assert_eq!(i, 1);
        // assert_eq!(r, SubmitNumberResponse { success: true, match_pair: None });

        // let (r, i) = client.submit_number(bob_submit_number).await.unwrap();
        // assert_eq!(i, 2);
        // assert_eq!(r, SubmitNumberResponse { success: true, match_pair: None });

        // let alice_cancel_number = CancelNumberRequest { address: alice, number: 42 };
        // let (r, i) = client.cancel_number(alice_cancel_number).await.unwrap();
        // assert_eq!(i, 3);
        // assert_eq!(r, CancelNumberResponse { success: true });

        // let alice_submit_number_second = SubmitNumberRequest { address: alice, number: 69 };
        // let (r, i) = client.submit_number(alice_submit_number_second).await.unwrap();
        // assert_eq!(i, 4);
        // assert_eq!(
        //     r,
        //     SubmitNumberResponse {
        //         success: true,
        //         match_pair: Some(MatchPair { user1: bob, user2: alice })
        //     }
        // );

        // // Give the batcher some time to process.
        // interval.tick().await;

        // // Check that partners have been updated on chain from the batch.
        // let partner = consumer_contract.getPartner(alice.into()).call().await.unwrap()._0;
        // assert_eq!(partner, bob);
        // let partner = consumer_contract.getPartner(bob.into()).call().await.unwrap()._0;
        // assert_eq!(partner, alice);
    }
    E2E::new().matching_game().run(test).await;
}
