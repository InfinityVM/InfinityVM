//! This module contains bindings for contracts that we interact with
//! View generated code with `cargo expand -p coprocessor-node --lib ::contracts`

#![allow(missing_docs)]

/// `IJobManager.sol` bindings
pub mod i_job_manager {
    alloy::sol! {
      #[sol(rpc)]
      "../../../contracts/src/IJobManager.sol"
    }
}

/// `JobManager.sol` bindings
pub mod job_manager {
    alloy::sol! {
      #[sol(rpc)]
      JobManager,
      "../../../contracts/out/JobManager.sol/JobManager.json"
    }
}
