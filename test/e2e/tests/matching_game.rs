#![allow(missing_docs)]

use alloy::{
    network::EthereumWallet,
    primitives::keccak256,
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use e2e::{Args, E2E};
use ivm_abi::{
    abi_encode_offchain_job_request, JobParams, OffchainResultWithMetadata,
    StatefulAppOnchainInput, StatefulAppResult,
};
use ivm_proto::{GetResultRequest, RelayStrategy, SubmitJobRequest, SubmitProgramRequest, VmType};
use kairos_trie::{stored::memory_db::MemoryDb, TrieRoot};
use matching_game_core::{
    api::{
        CancelNumberRequest, CancelNumberResponse, MatchPair, Request, SubmitNumberRequest,
        SubmitNumberResponse,
    },
    get_merkle_root_bytes, next_state,
};
use matching_game_programs::{MATCHING_GAME_ELF, MATCHING_GAME_PROGRAM_ID};
use matching_game_server::contracts::matching_game_consumer::MatchingGameConsumer;
use std::rc::Rc;
use tokio::time::{sleep, Duration};

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn state_job_submission_matching_game_consumer() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let matching_game = args.matching_game_consumer.unwrap();
        let program_id = MATCHING_GAME_PROGRAM_ID;
        let matching_game_signer_wallet =
            EthereumWallet::from(matching_game.matching_game_signer.clone());

        let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        let alice: [u8; 20] = alice_key.address().into();
        let bob: [u8; 20] = bob_key.address().into();

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MATCHING_GAME_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        let consumer_provider = ProviderBuilder::new()
            .wallet(matching_game_signer_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());

        let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
        let merkle_root0 = TrieRoot::Empty;

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];
        let (merkle_root1, _, _) = next_state(trie_db.clone(), merkle_root0, &requests1);

        let requests2 =
            vec![Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 })];
        let (merkle_root2, _, _) = next_state(trie_db.clone(), merkle_root1, &requests2);

        let requests3 =
            vec![Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 69 })];
        let (merkle_root3, _, _) = next_state(trie_db.clone(), merkle_root2, &requests3);

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
        interval.tick().await; // First tick processes immediately

        for (i, (requests, pre_txn_merkle_root, post_txn_merkle_root)) in [
            (requests1, merkle_root0, merkle_root1),
            (requests2, merkle_root1, merkle_root2),
            (requests3, merkle_root2, merkle_root3),
        ]
        .into_iter()
        .enumerate()
        {
            let nonce = (i + 1) as u64;
            let requests_bytes = bincode::serialize(&requests).unwrap();

            let (_, snapshot, _) = next_state(trie_db.clone(), pre_txn_merkle_root, &requests);
            let snapshot_bytes = bincode::serialize(&snapshot).unwrap();

            let mut combined_offchain_input = Vec::new();
            combined_offchain_input.extend_from_slice(&(requests_bytes.len() as u32).to_le_bytes());
            combined_offchain_input.extend_from_slice(&requests_bytes);
            combined_offchain_input.extend_from_slice(&snapshot_bytes);
            let offchain_input_hash = keccak256(&combined_offchain_input);

            let onchain_input = StatefulAppOnchainInput {
                input_state_root: get_merkle_root_bytes(pre_txn_merkle_root).into(),
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
            let job_request = SubmitJobRequest {
                request,
                signature,
                offchain_input: combined_offchain_input,
                relay_strategy: RelayStrategy::Ordered as i32,
            };
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
                assert_eq!(
                    *matching_game_output.output_state_root,
                    get_merkle_root_bytes(post_txn_merkle_root)
                );
            }
        }

        let consumer_contract =
            MatchingGameConsumer::new(matching_game.matching_game_consumer, &consumer_provider);
        let partner = consumer_contract.getPartner(alice.into()).call().await.unwrap()._0;
        assert_eq!(partner, bob);
        let partner = consumer_contract.getPartner(bob.into()).call().await.unwrap()._0;
        assert_eq!(partner, alice);
    }
    E2E::new().matching_game().run(test).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn matching_game_server_e2e() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let matching_game = args.matching_game_consumer.unwrap();
        let program_id = MATCHING_GAME_PROGRAM_ID;
        let matching_game_signer_wallet =
            EthereumWallet::from(matching_game.matching_game_signer.clone());
        let matching_game_endpoint = args.matching_game_endpoint.unwrap();

        let client = matching_game_server::client::Client::new(matching_game_endpoint);

        // Setup ready to use on chain accounts for Alice & Bob
        let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        let alice: [u8; 20] = alice_key.address().into();
        let bob: [u8; 20] = bob_key.address().into();

        // Seed coprocessor-node with ELF
        let submit_program_request = SubmitProgramRequest {
            program_elf: MATCHING_GAME_ELF.to_vec(),
            vm_type: VmType::Sp1.into(),
            program_id: program_id.to_vec(),
        };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        // Get chain state setup
        let consumer_provider = ProviderBuilder::new()
            .wallet(matching_game_signer_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract =
            MatchingGameConsumer::new(matching_game.matching_game_consumer, &consumer_provider);

        let alice_submit_number = SubmitNumberRequest { address: alice, number: 42 };
        let bob_submit_number = SubmitNumberRequest { address: bob, number: 69 };
        let (r, i) = client.submit_number(alice_submit_number).await.unwrap();
        assert_eq!(i, 1);
        assert_eq!(r, SubmitNumberResponse { success: true, match_pair: None });

        let (r, i) = client.submit_number(bob_submit_number).await.unwrap();
        assert_eq!(i, 2);
        assert_eq!(r, SubmitNumberResponse { success: true, match_pair: None });

        let alice_cancel_number = CancelNumberRequest { address: alice, number: 42 };
        let (r, i) = client.cancel_number(alice_cancel_number).await.unwrap();
        assert_eq!(i, 3);
        assert_eq!(r, CancelNumberResponse { success: true });

        let alice_submit_number_second = SubmitNumberRequest { address: alice, number: 69 };
        let (r, i) = client.submit_number(alice_submit_number_second).await.unwrap();
        assert_eq!(i, 4);
        assert_eq!(
            r,
            SubmitNumberResponse {
                success: true,
                match_pair: Some(MatchPair { user1: bob, user2: alice })
            }
        );

        sleep(Duration::from_secs(5)).await;

        // Check that partners have been updated on chain from the batch.
        let partner = consumer_contract.getPartner(alice.into()).call().await.unwrap()._0;
        assert_eq!(partner, bob);
        let partner = consumer_contract.getPartner(bob.into()).call().await.unwrap()._0;
        assert_eq!(partner, alice);
    }
    E2E::new().matching_game().run(test).await;
}
