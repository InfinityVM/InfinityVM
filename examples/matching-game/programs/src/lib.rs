use once_cell::sync::Lazy;
use sp1_utils::get_program_id;

pub const MATCHING_GAME_ELF: &[u8] =
    include_bytes!("../../../target/sp1/matching-game/matching-game-sp1-guest");

static MATCHING_GAME_PROGRAM_ID: Lazy<[u8; 32]> = Lazy::new(|| {
    get_program_id(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "../../../target/sp1/matching-game/matching-game-sp1-guest.vkey"
    ))
});

pub fn get_matching_game_program_id() -> [u8; 32] {
    *MATCHING_GAME_PROGRAM_ID
}

#[cfg(test)]
mod tests {
    use alloy::sol_types::SolType;
    use ivm_abi::{StatefulAppOnchainInput, StatefulAppResult};
    use ivm_zkvm::Zkvm;
    use kairos_trie::{stored::memory_db::MemoryDb, NodeHash, TrieRoot};
    use matching_game_core::{
        api::{CancelNumberRequest, Request, SubmitNumberRequest},
        get_merkle_root_bytes, next_state, Match, Matches,
    };
    use std::rc::Rc;

    #[test]
    fn submit_number_cancel_number() {
        let trie_db = Rc::new(MemoryDb::<Vec<u8>>::empty());
        let merkle_root0 = TrieRoot::Empty;

        let bob = [69u8; 20];
        let alice = [42u8; 20];
        let charlie = [55u8; 20];

        let requests1 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: alice, number: 42 }),
            Request::SubmitNumber(SubmitNumberRequest { address: bob, number: 69 }),
        ];
        let (merkle_root1, matching_game_out) =
            execute(requests1.clone(), trie_db.clone(), merkle_root0);
        assert_eq!(*matching_game_out.output_state_root, get_merkle_root_bytes(merkle_root1));
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert!(matches.is_empty());

        let requests2 = vec![
            Request::SubmitNumber(SubmitNumberRequest { address: charlie, number: 69 }),
            Request::CancelNumber(CancelNumberRequest { address: alice, number: 42 }),
        ];
        let (merkle_root2, matching_game_out) =
            execute(requests2.clone(), trie_db.clone(), merkle_root1);
        assert_eq!(*matching_game_out.output_state_root, get_merkle_root_bytes(merkle_root2));
        let matches = Matches::abi_decode(matching_game_out.result.as_ref(), false).unwrap();
        assert_eq!(matches, vec![Match { user1: bob.into(), user2: charlie.into() }]);
    }

    fn execute(
        requests: Vec<Request>,
        trie_db: Rc<MemoryDb<Vec<u8>>>,
        pre_txn_merkle_root: TrieRoot<NodeHash>,
    ) -> (TrieRoot<NodeHash>, StatefulAppResult) {
        let requests_bytes = bincode::serialize(&requests).unwrap();

        let (post_txn_merkle_root, snapshot, _) =
            next_state(trie_db.clone(), pre_txn_merkle_root, &requests);
        let snapshot_bytes = bincode::serialize(&snapshot).unwrap();

        let mut combined_offchain_input = Vec::new();
        combined_offchain_input.extend_from_slice(&(requests_bytes.len() as u32).to_le_bytes());
        combined_offchain_input.extend_from_slice(&requests_bytes);
        combined_offchain_input.extend_from_slice(&snapshot_bytes);

        let onchain_input = StatefulAppOnchainInput {
            input_state_root: get_merkle_root_bytes(pre_txn_merkle_root).into(),
            onchain_input: [0].into(),
        };

        let out_bytes = ivm_zkvm::Sp1
            .execute(
                super::MATCHING_GAME_ELF,
                <StatefulAppOnchainInput as SolType>::abi_encode(&onchain_input).as_slice(),
                &combined_offchain_input,
                32 * 1000 * 1000,
            )
            .unwrap();

        (
            post_txn_merkle_root,
            <StatefulAppResult as SolType>::abi_decode(&out_bytes, false).unwrap(),
        )
    }
}
