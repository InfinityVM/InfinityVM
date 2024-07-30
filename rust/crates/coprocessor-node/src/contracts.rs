//! This module contains bindings for contracts that we interact with
//! View generated code with `cargo expand -p coprocessor-node --lib ::contracts`

#![allow(missing_docs)]

/// `IJobManager` bindings
pub mod job_manager {
    alloy::sol! {
      #[sol(rpc)]
      "../../../contracts/src/IJobManager.sol"
    }
}
