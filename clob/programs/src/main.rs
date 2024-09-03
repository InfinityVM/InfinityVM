use risc0_zkvm::ExecutorEnv;
use risc0_zkvm::LocalProver;
use risc0_zkvm::Prover;

use alloy_sol_types::SolValue;
use clob_core::{
    api::{
        AddOrderRequest, CancelOrderRequest, ClobProgramInput, DepositRequest, Request,
        WithdrawRequest,
    },
    BorshKeccak256, ClobState,
};

fn main() {
    let init_state = ClobState::default();
    let bob = [69u8; 20];
    let alice = [42u8; 20];

    let requests = vec![
        Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
        Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
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
        // Buy 100 base for 4*100 quote, this will match with the first order
        Request::AddOrder(AddOrderRequest {
            address: bob,
            is_buy: true,
            limit_price: 4,
            size: 100,
        }),
        Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 }),
        Request::CancelOrder(CancelOrderRequest { oid: 1 }),
        Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 }),
    ];

    let input = ClobProgramInput {
        prev_state_hash: init_state.borsh_keccak256(),
        orders: borsh::to_vec(&requests).unwrap().into(),
    };
    let requests_encoded = input.abi_encode();
    let requests_encoded_len = requests_encoded.len() as u32;

    let state_encoded = borsh::to_vec(&init_state).unwrap();
    let state_encoded_len = state_encoded.len() as u32;

    let env = ExecutorEnv::builder()
        .session_limit(Some(32 * 1000 * 1000))
        .write(&state_encoded_len)
        .unwrap()
        .write_slice(&state_encoded)
        .write(&requests_encoded_len)
        .unwrap()
        .write_slice(&requests_encoded)
        .build()
        .unwrap();

    let prover = LocalProver::new("locals only");

    println!("clob zkvm prove/verify check: generating proof..");
    let prove_info = prover.prove(env, clob_programs::CLOB_ELF).unwrap();

    println!("clob zkvm prove/verify check: verifying proof..");
    prove_info.receipt.verify(clob_programs::CLOB_ID).unwrap();
}
