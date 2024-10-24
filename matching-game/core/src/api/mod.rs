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

/// All possible responses from the clob engine.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    /// [`SubmitNumberResponse`]
    SubmitNumber(SubmitNumberResponse),
    /// [`CancelNumberResponse`]
    CancelNumber(CancelNumberResponse),
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
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitNumberResponse {
    /// If the request was successfully processed.
    pub success: bool,
    /// The match that occurred.
    pub match_pair: Option<Match>,
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
