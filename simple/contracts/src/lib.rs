//! Contracts bindings for clob.

/// `ClobConsumer.sol` bindings
pub mod clob_consumer {
    alloy::sol! {
      #[sol(rpc)]
      SimpleConsumer,
      "../../contracts/out/SimpleConsumer.sol/SimpleConsumer.json"
    }
}
