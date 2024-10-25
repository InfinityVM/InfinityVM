//! Binding to ZKVM programs.

include!(concat!(env!("OUT_DIR"), "/methods.rs"));

#[cfg(test)]
mod tests {
    use alloy::{
        primitives::{keccak256, I256, U256},
        sol_types::{SolType, SolValue},
    };
    use matching_game_core::{
        api::{
            CancelNumberRequest, CancelNumberResponse, Match, Request, Response,
            SubmitNumberRequest, SubmitNumberResponse,
        },
        BorshKeccak256, Matches, MatchingGameState,
    };
    use matching_game_test_utils::next_state;

    use abi::{StatefulAppOnchainInput, StatefulAppResult};
    use zkvm::Zkvm;

    #[test]
    fn submit_number_cancel_number_execute() {
        let matching_game_state0 = MatchingGameState::default();
        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let charlie = [55u8; 20];

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];
        let matching_game_state1 = next_state(requests1.clone(), matching_game_state0.clone());
        let matching_game_out = execute(requests1.clone(), matching_game_state0.clone());
        assert_eq!(matching_game_out.output_state_root, matching_game_state1.borsh_keccak256());
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert!(matches.is_empty());

        let requests2 = vec![
            // Sell 100 base for 4*100 quote
            Request::SubmitNumber(SubmitNumberRequest { address: charlie, number: 69 }),
            Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 }),
        ];
        let matching_game_state2 = next_state(requests2.clone(), matching_game_state1.clone());
        let matching_game_out = execute(requests2.clone(), matching_game_state1.clone());
        assert_eq!(matching_game_out.output_state_root, matching_game_state2.borsh_keccak256());
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert_eq!(matches, vec![Match { user1: bob.into(), user2: charlie.into() }]);
    }

    fn execute(txns: Vec<Request>, init_state: MatchingGameState) -> StatefulAppResult {
        let requests_borsh = borsh::to_vec(&txns).expect("borsh works. qed.");
        let state_borsh = borsh::to_vec(&init_state).expect("borsh works. qed.");

        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_borsh.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_borsh);
        combined_offchain_input.extend_from_slice(&state_borsh);

        let state_hash = keccak256(&state_borsh);
        let onchain_input =
            StatefulAppOnchainInput { input_state_root: state_hash, onchain_input: [0].into() };

        let out_bytes = zkvm::Risc0 {}
            .execute(
                super::MATCHING_GAME_ELF,
                <StatefulAppOnchainInput as SolValue>::abi_encode(&onchain_input).as_slice(),
                &combined_offchain_input,
                32 * 1000 * 1000,
            )
            .unwrap();

        <StatefulAppResult as SolValue>::abi_decode(&out_bytes, true).unwrap()
    }
}
