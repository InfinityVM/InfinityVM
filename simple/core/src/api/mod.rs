//! Types in the public API

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

mod program;

pub use program::{ClobResultDeltas, DepositDelta, Diff, OrderDelta, WithdrawDelta};

/// All possible requests that can go into the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    /// [`SetValueRequest`]
    SetValue(SetValueRequest),
}

/// All possible responses from the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    /// [`SetValueResponse`]
    SetValue(SetValueResponse),
}

/// A response from the clob engine with the global index of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    /// Response to processing a request against the engine.
    pub response: Response,
    /// The global index of the request. The request is guaranteed to be processed
    /// via ordering indicated by this index.
    pub global_index: u64,
}

/// Sets a Value
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct SetValueRequest {
    /// Value to set
    pub value: String,
}

/// Response to [`AddOrderRequest`].
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct SetValueResponse {
    /// If the request was fully processed.
    pub success: bool,
}