//! Contracts bindings for matching game.

/// `MatchingGameConsumer.sol` bindings
pub mod matching_game_consumer {
    #![allow(missing_docs)]
    alloy::sol! {
      #[sol(rpc)]
      MatchingGameConsumer,
      "../../contracts/out/MatchingGameConsumer.sol/MatchingGameConsumer.json"
    }
}
