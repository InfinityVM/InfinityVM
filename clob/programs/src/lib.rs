//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    // use clob_core::{StfInput, StfOutput};
    use clob_core::{
        api::{AddOrderRequest, CancelOrderRequest, DepositRequest, Request, WithdrawRequest},
        tick, ClobState,
    };
    use risc0_zkvm::{Executor, ExecutorEnv, LocalProver};

    // TODO: fix this
    #[test]
    fn executes_program() {
        let zkvm_executor = LocalProver::new("locals only");

        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let requests1 = vec![
            Request::Deposit(DepositRequest { address: alice, base_free: 200, quote_free: 0 }),
            Request::Deposit(DepositRequest { address: bob, base_free: 0, quote_free: 800 }),
        ];

        let clob_state0 = ClobState::default();
        let inputs = [requests1].map(|rs| {
            let mut next_clob_state = clob_state0.clone();
            for r in rs.iter().cloned() {
                (_, next_clob_state, _) = tick(r, next_clob_state).unwrap();
            }

            (rs, next_clob_state)
        });

        for (txns, next_state) in inputs {
            let txns_b = borsh::to_vec(&txns).unwrap();

            // TODO: dynamically grab previous state
            let state_b = borsh::to_vec(&clob_state0).unwrap();

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

            let out_bytes = zkvm_executor.execute(env, super::CLOB_ELF).unwrap().journal.bytes;
        }
    }
}
