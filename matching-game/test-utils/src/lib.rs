use matching_game_core::{api::Request, tick, BorshKeccak256, MatchingGameState};

/// Returns the next state given a list of transactions.
pub fn next_state(txns: Vec<Request>, init_state: MatchingGameState) -> MatchingGameState {
    let mut next_matching_game_state = init_state;
    for tx in txns.iter().cloned() {
        (_, next_matching_game_state) = tick(tx, next_matching_game_state);
    }

    next_matching_game_state
}
