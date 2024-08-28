//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy_primitives::{I256, U256};
    use alloy_sol_types::SolValue;
    use clob_core::{
        api::{
            AddOrderRequest, CancelOrderRequest, ClobProgramOutput, DepositDelta, DepositRequest,
            OrderDelta, Request, WithdrawDelta, WithdrawRequest,
        },
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
        assert!(clob_out.withdraw_deltas.is_empty());
        assert!(clob_out.order_deltas.is_empty());
        assert_eq!(
            clob_out.deposit_deltas,
            vec![
                DepositDelta {
                    account: alice.into(),
                    base: U256::from(200),
                    quote: U256::from(0)
                },
                DepositDelta {
                    account: bob.into(),
                    base: U256::from(0),
                    quote: U256::from(800)
                },
            ]
        );

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
        assert!(clob_out.withdraw_deltas.is_empty());
        assert!(clob_out.deposit_deltas.is_empty());
        let a = OrderDelta {
            account: alice.into(),
            free_base: I256::try_from(-100i64).unwrap(),
            locked_base: I256::try_from(0).unwrap(),
            free_quote: I256::try_from(400).unwrap(),
            locked_quote: I256::try_from(0).unwrap(),
        };
        let b = OrderDelta {
            account: bob.into(),
            free_base: I256::try_from(100).unwrap(),
            locked_base: I256::try_from(0).unwrap(),
            free_quote: I256::try_from(-500i64).unwrap(),
            locked_quote: I256::try_from(100).unwrap(),
        };
        assert_eq!(clob_out.order_deltas, vec![a, b]);

        let requests3 = vec![
            Request::Withdraw(WithdrawRequest { address: alice, base_free: 100, quote_free: 400 }),
            Request::CancelOrder(CancelOrderRequest { oid: 1 }),
            Request::Withdraw(WithdrawRequest { address: bob, base_free: 100, quote_free: 400 }),
        ];
        let clob_state3 = next_state(requests3.clone(), clob_state2.clone());
        let clob_out = execute(requests3.clone(), clob_state2.clone(), &executor);
        assert_eq!(clob_out.next_state_hash, clob_state3.borsh_keccak256());
        assert!(clob_out.deposit_deltas.is_empty());
        let a = OrderDelta {
            account: bob.into(),
            free_base: I256::try_from(0).unwrap(),
            locked_base: I256::try_from(0).unwrap(),
            free_quote: I256::try_from(100).unwrap(),
            locked_quote: I256::try_from(-100i64).unwrap(),
        };
        assert_eq!(clob_out.order_deltas, vec![a]);
        let a = WithdrawDelta {
            account: alice.into(),
            base: U256::from(100),
            quote: U256::from(400),
        };
        let b = WithdrawDelta {
            account: bob.into(),
            base: U256::from(100),
            quote: U256::from(400),
        };
        assert_eq!(clob_out.withdraw_deltas, vec![a, b]);
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
