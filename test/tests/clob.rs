use alloy::{network::EthereumWallet, providers::ProviderBuilder};
use clob_contracts::clob_consumer::ClobConsumer;
use clob_core::{
    api::{AddOrderRequest, CancelOrderRequest, DepositRequest, Request, WithdrawRequest},
    ClobState,
};
use clob_programs::CLOB_ELF;
use e2e::{Args, E2E};
use proto::{SubmitProgramRequest, VmType};
use risc0_binfmt::compute_image_id;

fn program_id() -> Vec<u8> {
    compute_image_id(CLOB_ELF).unwrap().as_bytes().to_vec()
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
        let _consumer_contract = ClobConsumer::new(clob.clob_consumer, &consumer_provider);

        let clob_state = ClobState::default();

        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let requests1 = vec![
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        ];

        let requests2 = vec![
            // Sell 100 base for 4*100 quote
            Request::AddOrder(AddOrderRequest {
                address: alice,
                is_buy: false,
                limit_price: 4,
                size: 100,
            }),
            // Buy 100 base for 4*100 quote
            Request::AddOrder(AddOrderRequest {
                address: bob,
                is_buy: true,
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
        ];

        let requests3 = vec![
            // Sell 100 base for 4*100 quote
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 0, quote_free: 400 }),
            // Bob has 100 quote locked in order, 100 base free from fill, and lost 400 quote
            // in a fill.
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 100 }),
        ];

        let requests4 = vec![
            // TODO: should be either 4 or 5, check for off by 1
            // Bob cancels his order then never filled
            Request::CancelOrder(CancelOrderRequest { oid: 5 }),
            // Bob withdraws all of his funds
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 0, quote_free: 100 }),
        ];

        for requests in [requests1, requests2, requests3, requests4] {
            let _transactions_input = borsh::to_vec(&requests).unwrap();
            let _state_input = borsh::to_vec(&clob_state).unwrap();

            // TODO: logic to call state coprocessor endpoint
            // not sure the best way to create the next state state to plug into the program
            // could just run `tick` and compare the state hash from that to zkvm program response
        }

        // State should now be empty
    }
    E2E::new().clob().run(test).await;
}
