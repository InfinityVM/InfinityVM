//! Core Simple logic.
//! 

use std::sync::Arc;
use tokio::sync::RwLock;

use abi::StatefulProgramResult;
use alloy::{primitives::{keccak256, FixedBytes}, sol_types::SolValue};
use serde::{Deserialize, Serialize};
use borsh::{BorshDeserialize, BorshSerialize};
use api::{
    SetValueRequest, SetValueResponse, Request, Response,
};

pub mod api;

/// The state of the universe for the Simple App.
#[derive(
    Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshDeserialize, BorshSerialize,
)]
pub struct SimpleState {
    // The number of batches posted onchain.
    pub batch_num: u64,
    // The latest value set via offchain requests.
    pub latest_value: String,
}

impl SimpleState {
    /// Default instantiation
    pub fn default() -> Self {
        SimpleState {
            batch_num: 0,
            latest_value: "".to_string(),
        }
    }

    /// Gets the batch number
    pub const fn batch_num(&self) -> u64 {
        self.batch_num
    }

    /// Gets the latest value
    pub fn latest_value(&self) -> String {
        self.latest_value.clone()
    }
}

/// Sets the latest value in state
pub fn set_latest_value(req: SetValueRequest, state: Arc<RwLock<SimpleState>>) -> (SetValueResponse, SimpleState) {
    let mut write = state.try_write().unwrap(); // HACKY, cannot block or await here
    write.latest_value = req.value.clone();
    (SetValueResponse {
        success: true
    }, SimpleState {
        batch_num: 0,
        latest_value: req.value,
    })
}


alloy::sol! {
    /// Simple State
    #[derive(Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct StateResult {
        /// If the update succeeded.
        bool success;
    }
}

/// A tick will execute a single request against the CLOB state.
pub fn tick(request: Request, state: Arc<RwLock<SimpleState>>) -> (Response, SimpleState) {
    match request {
        Request::SetValue(req) => {
            let (resp, state) = set_latest_value(req, state);
            (Response::SetValue(resp), state)
        }
    }
}

/// Simple STF function
/// // TODO add requests back in
pub fn zkvm_stf(requests: Vec<Request>, mut state: SimpleState) -> StatefulProgramResult {

    for req in requests {
        let safe_state = Arc::new(RwLock::new(state));
        let (_, next_state) = tick(req, safe_state);
        state = next_state;
    }

    StatefulProgramResult {
        next_state_hash: state.borsh_keccak256(),
        // This type is not very friendly, unless you are using sol!
        result: StateResult::abi_encode(&StateResult{success:true}).into(),
    }
}

// TOOD: we should move all of this into a shared lib.
/// Trait for the sha256 hash of a borsh serialized type.
pub trait BorshSha256 {
    /// The sha256 hash of a borsh serialized type
    fn borsh_sha256(&self) -> [u8; 32];
}

// Blanket impl. for any type that implements borsh serialize.
impl<T: BorshSerialize> BorshSha256 for T {
    fn borsh_sha256(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        borsh::to_writer(&mut hasher, &self).expect("T is serializable");
        let hash = hasher.finalize();
        hash.into()
    }
}

/// Trait for the keccak256 hash of a borsh serialized type
pub trait BorshKeccak256 {
    /// The keccak256 hash of a borsh serialized type
    fn borsh_keccak256(&self) -> FixedBytes<32>;
}

impl<T: BorshSerialize> BorshKeccak256 for T {
    fn borsh_keccak256(&self) -> FixedBytes<32> {
        let borsh = borsh::to_vec(&self).expect("T is serializable");
        keccak256(&borsh)
    }
}