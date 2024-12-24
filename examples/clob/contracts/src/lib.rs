//! Contracts bindings for clob.

/// `ClobConsumer.sol` bindings
#[allow(clippy::too_many_arguments)]
pub mod clob_consumer {
    alloy::sol! {
      #[sol(rpc)]
      ClobConsumer,
      "../../../contracts/out/ClobConsumer.sol/ClobConsumer.json"
    }
}
