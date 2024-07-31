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

/// `TransparentUpgradeableProxy.sol` bindings
pub mod transparent_upgradeable_proxy {
    alloy::sol! {
      #[sol(rpc)]
      TransparentUpgradeableProxy,
      "../../../contracts/out/TransparentUpgradeableProxy.sol/TransparentUpgradeableProxy.json"
    }
}

/// `MockConsumer.sol` bindings
pub mod mock_consumer {
    alloy::sol! {
      #[sol(rpc)]
      MockConsumer,
      "../../../contracts/out/MockConsumer.sol/MockConsumer.json"
    }
}
