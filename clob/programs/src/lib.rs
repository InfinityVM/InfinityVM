//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy_primitives::{I256, U256};
    use alloy_sol_types::SolValue;
    use clob_core::{
        api::{
            AddOrderRequest, CancelOrderRequest, ClobProgramInput, ClobProgramOutput, DepositDelta,
            DepositRequest, OrderDelta, Request, WithdrawDelta, WithdrawRequest,
        },
        next_state, BorshKeccak256, ClobState,
    };

    use zkvm::Zkvm;

    #[test]
    fn deposit_create_cancel_withdraw() {
        let clob_state0 = ClobState::default();
        let bob = [69u8; 20];
        let alice = [42u8; 20];

        let requests1 = vec![
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        ];
        let clob_state1 = next_state(requests1.clone(), clob_state0.clone());
        let clob_out = execute(requests1.clone(), clob_state0.clone());
        assert_eq!(clob_out.next_state_hash, clob_state1.borsh_keccak256());
        assert!(clob_out.withdraw_deltas.is_empty());
        assert!(clob_out.order_deltas.is_empty());
        assert_eq!(
            clob_out.deposit_deltas,
            vec![
                DepositDelta { account: alice.into(), base: U256::from(200), quote: U256::from(0) },
                DepositDelta { account: bob.into(), base: U256::from(0), quote: U256::from(800) },
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
            // Buy 100 base for 4*100 quote, this will match with the first order
            Request::AddOrder(AddOrderRequest {
                address: bob,
                is_buy: true,
                limit_price: 4,
                size: 100,
            }),
        ];
        let clob_state2 = next_state(requests2.clone(), clob_state1.clone());
        let clob_out = execute(requests2.clone(), clob_state1.clone());
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
        let clob_out = execute(requests3.clone(), clob_state2.clone());
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
        let a =
            WithdrawDelta { account: alice.into(), base: U256::from(100), quote: U256::from(400) };
        let b =
            WithdrawDelta { account: bob.into(), base: U256::from(100), quote: U256::from(400) };
        assert_eq!(clob_out.withdraw_deltas, vec![a, b]);
    }

    fn execute(txns: Vec<Request>, init_state: ClobState) -> ClobProgramOutput {
        let input = ClobProgramInput {
            prev_state_hash: init_state.borsh_keccak256(),
            orders: borsh::to_vec(&txns).unwrap().into(),
        };

        let state_borsh = borsh::to_vec(&init_state).unwrap();
        let abi_input = input.abi_encode();
        let out_bytes = zkvm::Risc0 {}
            .execute_stateful(super::CLOB_ELF, &abi_input, &state_borsh, 32 * 1000 * 1000)
            .unwrap();

        ClobProgramOutput::abi_decode(&out_bytes, true).unwrap()
    }
}
