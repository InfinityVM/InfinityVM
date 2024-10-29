use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

mod program;

pub use program::Match;

/// All possible requests that can go into the matching game engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    /// [`SubmitNumberRequest`]
    SubmitNumber(SubmitNumberRequest),
    /// [`CancelNumberRequest`]
    CancelNumber(CancelNumberRequest),
}

/// All possible responses from the matching game engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    /// [`SubmitNumberResponse`]
    SubmitNumber(SubmitNumberResponse),
    /// [`CancelNumberResponse`]
    CancelNumber(CancelNumberResponse),
}

/// A response from the matching game engine with the global index of the request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize, BorshSerialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    /// Response to processing a request against the engine.
    pub response: Response,
    /// The global index of the request. The request is guaranteed to be processed
    /// via ordering indicated by this index.
    pub global_index: u64,
}
/// Submit a favorite number.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct SubmitNumberRequest {
    /// Account placing the order.
    pub address: [u8; 20],
    /// The number to submit.
    pub number: u64,
}

/// Cancel a favorite number submission.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct CancelNumberRequest {
    /// Account cancelling the number submission.
    pub address: [u8; 20],
    /// The number to cancel.
    pub number: u64,
}

/// Response to [`SubmitNumberRequest`].
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct SubmitNumberResponse {
    /// If the request was successfully processed.
    pub success: bool,
}

/// Response to [`CancelNumberRequest`].
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
#[serde(rename_all = "camelCase")]
pub struct CancelNumberResponse {
    /// If the request was successfully processed.
    pub success: bool,
}
