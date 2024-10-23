// TODO: Remove unused_imports after uncommenting tests.
// [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
#![allow(unused_imports)]

use abi::{abi_encode_offchain_job_request, JobParams, StatefulAppResult};
use alloy::{
    network::EthereumWallet,
    primitives::U256,
    providers::ProviderBuilder,
    signers::{local::PrivateKeySigner, Signer},
    sol_types::SolValue,
};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::{
    api::{
        AddOrderRequest, AddOrderResponse, AssetBalance, CancelOrderRequest, DepositRequest,
        FillStatus, OrderFill, Request, WithdrawRequest,
    },
    BorshKeccak256, ClobState,
};
use clob_programs::CLOB_ELF;
use clob_test_utils::{mint_and_approve, mock_erc20::MockErc20, next_state};
use e2e::{Args, E2E};
use proto::{GetResultRequest, SubmitJobRequest, SubmitProgramRequest, VmType};
use risc0_binfmt::compute_image_id;
use tokio::time::{sleep, Duration};
use zkvm_executor::service::OffchainResultWithMetadata;

// TODO: Remove dead_code after uncommenting tests.
// [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
#[allow(dead_code)]
fn program_id() -> Vec<u8> {
    compute_image_id(CLOB_ELF).unwrap().as_bytes().to_vec()
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn state_job_submission_clob_consumer() {
    // TODO: Uncomment this test after updating CLOB to pass in state root + merkle proofs
    // to coprocessor instead of the entire state.
    // [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
    async fn test(mut _args: Args) {
        // let anvil = args.anvil;
        // let clob = args.clob_consumer.unwrap();
        // let program_id = program_id();
        // let clob_signer_wallet = EthereumWallet::from(clob.clob_signer.clone());
        // let clob_state0 = ClobState::default();

        // let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        // let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        // let alice: [u8; 20] = alice_key.address().into();
        // let bob: [u8; 20] = bob_key.address().into();
        // let alice_wallet = EthereumWallet::new(alice_key);
        // let bob_wallet = EthereumWallet::new(bob_key);

        // // Seed coprocessor-node with ELF
        // let submit_program_request =
        //     SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into()
        // }; let submit_program_response = args
        //     .coprocessor_node
        //     .submit_program(submit_program_request)
        //     .await
        //     .unwrap()
        //     .into_inner();
        // assert_eq!(submit_program_response.program_id, program_id);

        // let consumer_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(clob_signer_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let base_contract = MockErc20::new(clob.base_erc20, &consumer_provider);
        // let quote_contract = MockErc20::new(clob.quote_erc20, &consumer_provider);

        // let call = base_contract.mint(alice.into(), U256::from(1_000));
        // let r1 = call.send().await.unwrap().get_receipt();
        // let call = quote_contract.mint(bob.into(), U256::from(1_000));
        // let r2 = call.send().await.unwrap().get_receipt();

        // let alice_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(alice_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let alice_base = MockErc20::new(clob.base_erc20, &alice_provider);
        // let call = alice_base.approve(clob.clob_consumer, U256::from(1_000));
        // let r3 = call.send().await.unwrap().get_receipt();

        // let alice_contract = ClobConsumer::new(clob.clob_consumer, &alice_provider);
        // let call = alice_contract.deposit(U256::from(200), U256::from(0));
        // let r4 = call.send().await.unwrap().get_receipt();

        // let bob_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(bob_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let bob_quote = MockErc20::new(clob.quote_erc20, &bob_provider);
        // let call = bob_quote.approve(clob.clob_consumer, U256::from(1_000));
        // let r5 = call.send().await.unwrap().get_receipt();

        // let bob_contract = ClobConsumer::new(clob.clob_consumer, &bob_provider);
        // let call = bob_contract.deposit(U256::from(0), U256::from(800));
        // let r6 = call.send().await.unwrap().get_receipt();
        // tokio::try_join!(r1, r2, r3, r4, r5, r6).unwrap();

        // let bob_quote_bal = bob_quote.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_quote_bal, U256::from(200));
        // let alice_base_bal = alice_base.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_base_bal, U256::from(800));

        // // Wait for CLOB node and coprocessor to process deposits and send batch to contracts.
        // // This is necessary because the CLOB node will automatically pick up the deposit events.
        // sleep(Duration::from_secs(10)).await;

        // let requests1 = vec![
        //     Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
        //     Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        // ];
        // let clob_state1 = next_state(requests1.clone(), clob_state0.clone());

        // let requests2 = vec![
        //     // Sell 100 base for 4*100 quote
        //     Request::AddOrder(AddOrderRequest {
        //         address: alice,
        //         is_buy: false,
        //         limit_price: 4,
        //         size: 100,
        //     }),
        //     // Buy 100 base for 1*100 quote, this won't match but will lock funds
        //     Request::AddOrder(AddOrderRequest {
        //         address: bob,
        //         is_buy: true,
        //         limit_price: 1,
        //         size: 100,
        //     }),
        //     // Buy 100 base for 4*100 quote
        //     Request::AddOrder(AddOrderRequest {
        //         address: bob,
        //         is_buy: true,
        //         limit_price: 4,
        //         size: 100,
        //     }),
        // ];
        // let clob_state2 = next_state(requests2.clone(), clob_state1.clone());

        // let requests3 = vec![
        //     Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400
        // }),     Request::CancelOrder(CancelOrderRequest { oid: 1 }),
        //     Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 }),
        // ];
        // let clob_state3 = next_state(requests3.clone(), clob_state2.clone());

        // let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));
        // interval.tick().await; // First tick processes immediately
        // let mut nonce = 2;
        // for (requests, init_state, next_state) in
        //     [(requests2, &clob_state1, &clob_state2), (requests3, &clob_state2, &clob_state3)]
        // {
        //     let offchain_input_hash = requests.borsh_keccak256();
        //     let offchain_input_borsh = borsh::to_vec(&requests).unwrap();

        //     let previous_state_hash = init_state.borsh_keccak256();
        //     let state_borsh = borsh::to_vec(&init_state).unwrap();

        //     let params = JobParams {
        //         nonce,
        //         max_cycles: 32 * 1000 * 1000,
        //         consumer_address: **clob.clob_consumer,
        //         onchain_input: &[],
        //         offchain_input_hash: offchain_input_hash.into(),
        //         program_id: &program_id,
        //     };
        //     let request = abi_encode_offchain_job_request(params.clone());
        //     let signature =
        //         clob.clob_signer.sign_message(&request).await.unwrap().as_bytes().to_vec();
        //     let job_request = SubmitJobRequest {
        //         request,
        //         signature,
        //         offchain_input: offchain_input_borsh,
        //     };
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
        //         let clob_output = StatefulAppResult::abi_decode(&raw_output, true).unwrap();
        //         assert_eq!(clob_output.output_state_root, next_state.borsh_keccak256());
        //     }

        //     nonce += 1;
        // }

        // let bob_quote_bal = bob_quote.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_quote_bal, U256::from(600));
        // let bob_base_bal = alice_base.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_base_bal, U256::from(100));

        // let alice_quote_bal = bob_quote.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_quote_bal, U256::from(400));
        // let alice_base_bal = alice_base.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_base_bal, U256::from(900));
    }
    E2E::new().clob().run(test).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn clob_node_e2e() {
    // TODO: Uncomment this test after updating CLOB to pass in state root + merkle proofs
    // to coprocessor instead of the entire state.
    // [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
    async fn test(mut _args: Args) {
        // let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(13));
        // interval.tick().await; // First tick processes immediately

        // let anvil = args.anvil;
        // let clob = args.clob_consumer.unwrap();
        // let program_id = program_id();
        // let clob_signer_wallet = EthereumWallet::from(clob.clob_signer.clone());
        // let clob_endpoint = args.clob_endpoint.unwrap();

        // let client = clob_client::Client::new(clob_endpoint);

        // // Setup ready to use on chain accounts for Alice & Bob
        // let alice_key: PrivateKeySigner = anvil.anvil.keys()[8].clone().into();
        // let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        // let alice: [u8; 20] = alice_key.address().into();
        // let bob: [u8; 20] = bob_key.address().into();
        // let alice_wallet = EthereumWallet::new(alice_key);
        // let bob_wallet = EthereumWallet::new(bob_key);

        // // Seed coprocessor-node with ELF
        // let submit_program_request =
        //     SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into()
        // }; let submit_program_response = args
        //     .coprocessor_node
        //     .submit_program(submit_program_request)
        //     .await
        //     .unwrap()
        //     .into_inner();
        // assert_eq!(submit_program_response.program_id, program_id);

        // // Get chain state setup
        // let consumer_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(clob_signer_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let consumer_contract = ClobConsumer::new(clob.clob_consumer, &consumer_provider);
        // let base_contract = MockErc20::new(clob.base_erc20, &consumer_provider);
        // let quote_contract = MockErc20::new(clob.quote_erc20, &consumer_provider);

        // // Mint erc20 tokens to Alice and Bob so they have funds to deposit on the exchange
        // let call = base_contract.mint(alice.into(), U256::from(1_000));
        // let r1 = call.send().await.unwrap().get_receipt();
        // let call = quote_contract.mint(bob.into(), U256::from(1_000));
        // let r2 = call.send().await.unwrap().get_receipt();

        // // Alice and Bob both approve the ClobConsumer to move the ERC20 and then deposit
        // let alice_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(alice_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let alice_base = MockErc20::new(clob.base_erc20, &alice_provider);
        // let call = alice_base.approve(clob.clob_consumer, U256::from(1_000));
        // let r3 = call.send().await.unwrap().get_receipt();

        // let alice_contract = ClobConsumer::new(clob.clob_consumer, &alice_provider);
        // let call = alice_contract.deposit(U256::from(200), U256::from(0));
        // let r4 = call.send().await.unwrap().get_receipt();

        // let bob_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(bob_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let bob_quote = MockErc20::new(clob.quote_erc20, &bob_provider);
        // let call = bob_quote.approve(clob.clob_consumer, U256::from(1_000));
        // let r5 = call.send().await.unwrap().get_receipt();

        // let bob_contract = ClobConsumer::new(clob.clob_consumer, &bob_provider);
        // let call = bob_contract.deposit(U256::from(0), U256::from(800));
        // let r6 = call.send().await.unwrap().get_receipt();

        // // Wait for all the transactions to hit the chain
        // tokio::try_join!(r1, r2, r3, r4, r5, r6).unwrap();

        // // Sanity check that the ERC20s transferred
        // let bob_quote_bal = bob_quote.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_quote_bal, U256::from(200));
        // let alice_base_bal = alice_base.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_base_bal, U256::from(800));

        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.base_balances().get(&alice).unwrap(),
        //     AssetBalance { free: 200, locked: 0 }
        // );
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 800, locked: 0 }
        // );

        // let alice_limit =
        //     AddOrderRequest { address: alice, is_buy: false, limit_price: 4, size: 100 };
        // let (r, i) = client.order(alice_limit).await.unwrap();
        // // i is 3 here because the CLOB node automatically picks up the deposit
        // // events from the contracts earlier (one each for Alice and Bob).
        // assert_eq!(i, 3);
        // assert_eq!(
        //     r,
        //     AddOrderResponse {
        //         success: true,
        //         status: Some(FillStatus {
        //             oid: 0,
        //             size: 100,
        //             address: alice,
        //             filled_size: 0,
        //             fills: vec![]
        //         })
        //     }
        // );

        // let bob_limit1 = AddOrderRequest { address: bob, is_buy: true, limit_price: 1, size: 100
        // }; let (r, i) = client.order(bob_limit1).await.unwrap();
        // assert_eq!(i, 4);
        // assert_eq!(
        //     r,
        //     AddOrderResponse {
        //         success: true,
        //         status: Some(FillStatus {
        //             oid: 1,
        //             size: 100,
        //             address: bob,
        //             filled_size: 0,
        //             fills: vec![]
        //         })
        //     }
        // );

        // let bob_limit2 = AddOrderRequest { address: bob, is_buy: true, limit_price: 4, size: 100
        // }; let (r, i) = client.order(bob_limit2).await.unwrap();
        // assert_eq!(i, 5);
        // assert_eq!(
        //     r,
        //     AddOrderResponse {
        //         success: true,
        //         status: Some(FillStatus {
        //             oid: 2,
        //             size: 100,
        //             address: bob,
        //             filled_size: 100,
        //             fills: vec![OrderFill {
        //                 maker_oid: 0,
        //                 taker_oid: 2,
        //                 buyer: bob,
        //                 seller: alice,
        //                 price: 4,
        //                 size: 100
        //             }]
        //         })
        //     }
        // );
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.base_balances().get(&alice).unwrap(),
        //     AssetBalance { free: 100, locked: 0 }
        // );
        // assert_eq!(
        //     *state.quote_balances().get(&alice).unwrap(),
        //     AssetBalance { free: 400, locked: 0 }
        // );
        // assert_eq!(
        //     *state.base_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 100, locked: 0 }
        // );
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 300, locked: 100 }
        // );

        // // Give the batcher some time to process.
        // interval.tick().await;

        // // Check that balances have been updated on chain from the batch.
        // let bob_free_base =
        // consumer_contract.freeBalanceBase(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_free_base, U256::from(100));
        // let bob_free_quote =
        //     consumer_contract.freeBalanceQuote(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_free_quote, U256::from(300));

        // let alice_withdraw = WithdrawRequest { address: alice, base_free: 100, quote_free: 400 };
        // let (_, i) = client.withdraw(alice_withdraw).await.unwrap();
        // assert_eq!(i, 6);
        // let state = client.clob_state().await.unwrap();
        // assert!(!state.quote_balances().contains_key(&alice));
        // assert!(!state.base_balances().contains_key(&alice));

        // let bob_cancel = CancelOrderRequest { oid: 1 };
        // let (_, i) = client.cancel(bob_cancel).await.unwrap();
        // assert_eq!(i, 7);
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 400, locked: 0 }
        // );

        // let bob_withdraw = WithdrawRequest { address: bob, base_free: 100, quote_free: 400 };
        // let (_, i) = client.withdraw(bob_withdraw).await.unwrap();
        // assert_eq!(i, 8);
        // let state = client.clob_state().await.unwrap();
        // assert!(state.quote_balances().is_empty());
        // assert!(state.base_balances().is_empty());

        // // Wait for batches to hit the chain
        // interval.tick().await;

        // let bob_quote_bal = bob_quote.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_quote_bal, U256::from(600));
        // let bob_base_bal = alice_base.balanceOf(bob.into()).call().await.unwrap()._0;
        // assert_eq!(bob_base_bal, U256::from(100));

        // let alice_quote_bal = bob_quote.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_quote_bal, U256::from(400));
        // let alice_base_bal = alice_base.balanceOf(alice.into()).call().await.unwrap()._0;
        // assert_eq!(alice_base_bal, U256::from(900));
    }
    E2E::new().clob().run(test).await;
}

// Test that we can partially match orders, withdraw, and then cancel a partially matched order. In
// the past the clob has panicked in this edge case
#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn cancel_works() {
    // TODO: Uncomment this test after updating CLOB to pass in state root + merkle proofs
    // to coprocessor instead of the entire state.
    // [ref]: https://github.com/InfinityVM/InfinityVM/issues/320
    async fn test(_args: Args) {
        // let clob = args.clob_consumer.unwrap();
        // let anvil = args.anvil;

        // let clob_endpoint = args.clob_endpoint.unwrap();
        // let client = clob_client::Client::new(clob_endpoint);

        // mint_and_approve(&clob, anvil.anvil.endpoint(), 10).await;

        // let bob_key: PrivateKeySigner = anvil.anvil.keys()[9].clone().into();
        // let bob: [u8; 20] = bob_key.address().into();
        // let bob_wallet = EthereumWallet::new(bob_key);

        // let bob_provider = ProviderBuilder::new()
        //     .with_recommended_fillers()
        //     .wallet(bob_wallet)
        //     .on_http(anvil.anvil.endpoint().parse().unwrap());
        // let bob_contract = ClobConsumer::new(clob.clob_consumer, &bob_provider);
        // let call = bob_contract.deposit(U256::from(150), U256::from(300));
        // call.send().await.unwrap().get_receipt().await.unwrap();

        // // Wait for deposits to process
        // sleep(Duration::from_secs(3)).await;
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.base_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 150, locked: 0 }
        // );
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 300, locked: 0 }
        // );

        // let limit1 = AddOrderRequest { address: bob, is_buy: false, limit_price: 5, size: 1 };

        // let (_r, _i) = client.order(limit1).await.unwrap();
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.base_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 149, locked: 1 }
        // );
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 300, locked: 0 }
        // );

        // let limit2 = AddOrderRequest { address: bob, is_buy: true, limit_price: 6, size: 2 };
        // let (r, _i) = client.order(limit2).await.unwrap();
        // assert_eq!(
        //     r,
        //     AddOrderResponse {
        //         success: true,
        //         status: Some(FillStatus {
        //             oid: 1,
        //             size: 2,
        //             address: [
        //                 160, 238, 122, 20, 45, 38, 124, 31, 54, 113, 78, 74, 143, 117, 97, 47,
        // 32,                 167, 151, 32
        //             ],
        //             filled_size: 1,
        //             fills: vec![OrderFill {
        //                 maker_oid: 0,
        //                 taker_oid: 1,
        //                 size: 1,
        //                 price: 5,
        //                 buyer: [
        //                     160, 238, 122, 20, 45, 38, 124, 31, 54, 113, 78, 74, 143, 117, 97,
        // 47,                     32, 167, 151, 32
        //                 ],
        //                 seller: [
        //                     160, 238, 122, 20, 45, 38, 124, 31, 54, 113, 78, 74, 143, 117, 97,
        // 47,                     32, 167, 151, 32
        //                 ]
        //             }]
        //         })
        //     }
        // );
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(
        //     *state.base_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 150, locked: 0 }
        // );
        // // (6 * 2)(taker price*size) - (5 * 1)(maker price*size)
        // assert_eq!(
        //     *state.quote_balances().get(&bob).unwrap(),
        //     AssetBalance { free: 293, locked: 7 }
        // );

        // let withdraw = WithdrawRequest { address: bob, base_free: 150, quote_free: 293 };
        // let (_r, _i) = client.withdraw(withdraw).await.unwrap();
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(*state.base_balances().get(&bob).unwrap(), AssetBalance { free: 0, locked: 0
        // }); assert_eq!(*state.quote_balances().get(&bob).unwrap(), AssetBalance { free:
        // 0, locked: 7 });

        // // Cancel the partially filled order
        // let cancel = CancelOrderRequest { oid: 1 };
        // let (_r, _i) = client.cancel(cancel).await.unwrap();
        // let state = client.clob_state().await.unwrap();
        // assert_eq!(*state.base_balances().get(&bob).unwrap(), AssetBalance { free: 0, locked: 0
        // }); assert_eq!(*state.quote_balances().get(&bob).unwrap(), AssetBalance { free:
        // 7, locked: 0 });
    }
    E2E::new().clob().run(test).await;
}
