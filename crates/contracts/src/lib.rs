//! This module contains bindings for contracts that we interact with.
//!
//! You can view generated code docs with `make doc`.
// The `make contracts` command should copy the relevant json file into place
// after building the json abi from source.

#![allow(missing_docs)]

use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

/// `IJobManager.sol` bindings
pub mod i_job_manager {
    alloy::sol! {
      #[sol(rpc)]
      IJobManager,
      "json/IJobManager.json"
    }
}

/// `JobManager.sol` bindings
pub mod job_manager {
    alloy::sol! {
      #[sol(rpc)]
      JobManager,
      "json/JobManager.json"
    }
}

/// `TransparentUpgradeableProxy.sol` bindings
pub mod transparent_upgradeable_proxy {
    alloy::sol! {
      #[sol(rpc)]
      TransparentUpgradeableProxy,
      "json/TransparentUpgradeableProxy.json"
    }
}

/// `ProxyAdmin.sol` bindings
pub mod proxy_admin {
    alloy::sol! {
      #[sol(rpc)]
      ProxyAdmin,
      "json/ProxyAdmin.json"
    }
}

/// `Consumer.sol` bindings
pub mod consumer {
    alloy::sol! {
      #[sol(rpc)]
      Consumer,
      "json/Consumer.json"
    }
}

/// `MockConsumer.sol` bindings
pub mod mock_consumer {
    alloy::sol! {
      #[sol(rpc)]
      MockConsumer,
      "json/MockConsumer.json"
    }
}

/// Path to write deploy info to
pub const DEFAULT_DEPLOY_INFO: &str = "./logs/deploy_info.json";

/// Contract deployment info.
#[derive(Serialize, Deserialize, Debug)]
pub struct DeployInfo {
    /// Job Manager contract address.
    pub job_manager: Address,
    /// Quote ERC20 contract address.
    pub quote_erc20: Address,
    /// Base ERC20 contract address.
    pub base_erc20: Address,
    /// CLOB Consumer contract address.
    pub clob_consumer: Address,
    /// Mock Consumer contract address.
    pub mock_consumer: Address,
}

pub fn get_default_deploy_info() -> eyre::Result<DeployInfo> {
    let filename = DEFAULT_DEPLOY_INFO.to_string();
    let raw_json = std::fs::read(filename)?;
    serde_json::from_slice(&raw_json).map_err(Into::into)
}
