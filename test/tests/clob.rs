use alloy::{network::EthereumWallet, providers::ProviderBuilder, signers::Signer};
use alloy_sol_types::SolValue;
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::api::ClobProgramOutput;
use clob_core::{
    api::{
        AddOrderRequest, CancelOrderRequest, ClobProgramInput, DepositRequest, Request,
        WithdrawRequest,
    },
    tick, BorshKeccack256, ClobState,
};
use clob_programs::CLOB_ELF;
use e2e::{Args, E2E};
use proto::GetResultRequest;
use proto::{SubmitJobRequest, SubmitProgramRequest, VmType};
use risc0_binfmt::compute_image_id;
use zkvm_executor::service::ResultWithMetadata;

// TODO: these should be in some sort of sdk-primitives crate
use clob_contracts::{abi_encode_offchain_job_request, JobParams};

fn program_id() -> Vec<u8> {
    compute_image_id(CLOB_ELF).unwrap().as_bytes().to_vec()
}

fn next_state(txns: Vec<Request>, init_state: ClobState) -> ClobState {
    let mut next_clob_state = init_state;
    for tx in txns.iter().cloned() {
        (_, next_clob_state, _) = tick(tx, next_clob_state).unwrap();
    }

    next_clob_state
}

#[tokio::test]
#[ignore]
async fn state_job_submission_clob_consumer() {
    async fn test(mut args: Args) {
        let anvil = args.anvil;
        let clob = args.clob_consumer.unwrap();
        let program_id = program_id();

        let clob_signer_wallet = EthereumWallet::from(clob.clob_signer.clone());

        // Seed coprocessor-node with ELF
        let submit_program_request =
            SubmitProgramRequest { program_elf: CLOB_ELF.to_vec(), vm_type: VmType::Risc0.into() };
        let submit_program_response = args
            .coprocessor_node
            .submit_program(submit_program_request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(submit_program_response.program_id, program_id);

        let consumer_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(clob_signer_wallet)
            .on_http(anvil.anvil.endpoint().parse().unwrap());
        let consumer_contract = ClobConsumer::new(clob.clob_consumer, &consumer_provider);

        let clob_state0 = ClobState::default();
        let init_state_hash: [u8; 32] = clob_state0.borsh_keccak256().into();
        let call = consumer_contract.setLatestStateRootHash(init_state_hash.into());
        let _ = call.send().await.unwrap().get_receipt().await.unwrap();

        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let requests1 = vec![
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        ];
        let clob_state1 = next_state(requests1.clone(), clob_state0.clone());

        let requests2 = vec![
            // Sell 100 base for 4*100 quote
            Request::AddOrder(AddOrderRequest {
                address: alice,
                is_buy: false,
                limit_price: 4,
                size: 100,
            }),
            // Buy 100 base for 1*100 quote, this won't match but will lock funds
            Request::AddOrder(AddOrderRequest {
                address: bob,
                is_buy: true,
                limit_price: 1,
                size: 100,
            }),
            // Buy 100 base for 4*100 quote
            Request::AddOrder(AddOrderRequest {
                address: bob,
                is_buy: true,
                limit_price: 4,
                size: 100,
            }),
        ];
        let clob_state2 = next_state(requests2.clone(), clob_state1.clone());

        let requests3 = vec![
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 }),
            Request::CancelOrder(CancelOrderRequest { oid: 1 }),
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 }),
        ];
        let clob_state3 = next_state(requests3.clone(), clob_state2.clone());

        let mut nonce = 420;
        for (txns, init_state, next_state) in [
            (requests1, &clob_state0, &clob_state1),
            (requests2, &clob_state1, &clob_state2),
            (requests3, &clob_state2, &clob_state3),
        ] {
            dbg!(nonce);
            let input = ClobProgramInput {
                prev_state_hash: init_state.borsh_keccak256(),
                orders: borsh::to_vec(&txns).unwrap().into(),
            };

            let state_borsh = borsh::to_vec(&init_state).unwrap();
            let input_abi = input.abi_encode();

            let params = JobParams {
                nonce,
                max_cycles: 32 * 1000 * 1000,
                consumer_address: **clob.clob_consumer,
                program_input: input_abi,
                program_id: program_id.clone(),
            };
            let request = abi_encode_offchain_job_request(params.clone());
            let signature =
                clob.clob_signer.sign_message(&request).await.unwrap().as_bytes().to_vec();
            let job_request = SubmitJobRequest { request, signature, program_state: state_borsh };
            let submit_job_response =
                args.coprocessor_node.submit_job(job_request).await.unwrap().into_inner();

            // Wait for the job to be processed
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

            let job_id = submit_job_response.job_id;
            let result_with_metadata = args
                .coprocessor_node
                .get_result(GetResultRequest { job_id })
                .await
                .unwrap()
                .into_inner()
                .job_result
                .unwrap()
                .result_with_metadata;

            let raw_output = {
                use alloy::sol_types::SolType;
                dbg!(result_with_metadata.len());
                let abi_decoded_output =
                    ResultWithMetadata::abi_decode_params(&result_with_metadata, false).unwrap();
                abi_decoded_output.4
            };

            {
                let clob_output = ClobProgramOutput::abi_decode(&raw_output, true).unwrap();
                assert_eq!(clob_output.next_state_hash, next_state.borsh_keccak256());
            }

            nonce += 69;
        }
    }
    E2E::new().clob().run(test).await;
}