//! Contracts bindings for clob.

/// `ClobConsumer.sol` bindings
pub mod clob_consumer {
    alloy::sol! {
      #[sol(rpc)]
      ClobConsumer,
      "../../contracts/out/ClobConsumer.sol/ClobConsumer.json"
    }
}
