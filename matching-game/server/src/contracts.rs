//! Contracts bindings for matching game.

/// `MatchingGameConsumer.sol` bindings
pub mod matching_game_consumer {
    alloy::sol! {
      #[sol(rpc)]
      MatchingGameConsumer,
      "../../contracts/out/MatchingGameConsumer.sol/MatchingGameConsumer.json"
    }
}
