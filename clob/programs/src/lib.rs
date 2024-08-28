//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy_sol_types::SolValue;
    use clob_core::api::AddOrderRequest;
    use clob_core::api::WithdrawRequest;
    use clob_core::{
        api::{CancelOrderRequest, ClobProgramOutput, DepositRequest, Request},
        tick, BorshKeccack256, ClobState,
    };
    use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};

    #[test]
    fn executes_program() {
        let executor = LocalProver::new("locals only");

        let clob_state0 = ClobState::default();
        let bob = [69u8; 20];
        let alice = [42u8; 20];

        let requests1 = vec![
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        ];
        let clob_state1 = next_state(requests1.clone(), clob_state0.clone());
        // Deposits
        let clob_out = execute(requests1.clone(), clob_state0.clone(), &executor);
        assert_eq!(clob_out.next_state_hash, clob_state1.borsh_keccak256());

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
        let clob_out = execute(requests2.clone(), clob_state1.clone(), &executor);
        assert_eq!(clob_out.next_state_hash, clob_state2.borsh_keccak256());

        let requests3 = vec![
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 }),
            Request::CancelOrder(CancelOrderRequest { oid: 1 }),
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 }),
        ];
        let clob_state3 = next_state(requests3.clone(), clob_state2.clone());
        let clob_out = execute(requests3.clone(), clob_state2.clone(), &executor);
        assert_eq!(clob_out.next_state_hash, clob_state3.borsh_keccak256());
    }

    fn next_state(txns: Vec<Request>, init_state: ClobState) -> ClobState {
        let mut next_clob_state = init_state;
        for tx in txns.iter().cloned() {
            (_, next_clob_state, _) = tick(tx, next_clob_state).unwrap();
        }

        next_clob_state
    }

    fn execute(
        txns: Vec<Request>,
        init_state: ClobState,
        executor: &LocalProver,
    ) -> ClobProgramOutput {
        let txns_b = borsh::to_vec(&txns).unwrap();
        let state_b = borsh::to_vec(&init_state).unwrap();
        let txns_len = txns_b.len() as u32;
        let state_len = state_b.len() as u32;
        let env = ExecutorEnv::builder()
            .write::<u32>(&state_len)
            .unwrap()
            .write_slice(&state_b)
            .write(&txns_len)
            .unwrap()
            .write_slice(&txns_b)
            .build()
            .unwrap();
        let out_bytes = executor.execute(env, super::CLOB_ELF).unwrap().journal.bytes;

        ClobProgramOutput::abi_decode(&out_bytes, true).unwrap()
    }
}
