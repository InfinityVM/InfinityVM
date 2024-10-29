//! Core logic and types of the matching game.
//!
//! Note that everything in here needs to be able to target the ZKVM architecture.

use std::collections::HashMap;

use alloy::{
    primitives::{utils::keccak256, FixedBytes},
    sol,
    sol_types::SolType,
};

use abi::StatefulAppResult;
use api::{
    CancelNumberRequest, CancelNumberResponse, Request, Response, SubmitNumberRequest,
    SubmitNumberResponse,
};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// Matching game server API types.
pub mod api;

pub use crate::api::Match;

/// Simple tick function that processes a request and returns a response.
pub fn tick<'a>(request: Request) -> Response {
    match request {
        Request::SubmitNumber(_) => Response::SubmitNumber(SubmitNumberResponse { success: true }),
        Request::CancelNumber(_) => Response::CancelNumber(CancelNumberResponse { success: true }),
    }
}

/// Array of matches. Result from matching game program.
pub type Matches = sol! {
    Match[]
};
