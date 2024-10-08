//! The proto generated server, client, and messages.
//!
//! You can view generated code docs with `make doc`.

// We don't have control over tonic generated code so we ignore the
// lints it complains about
// #![allow(clippy::all, clippy::missing_const_for_fn, unreachable_pub)]

use serde_with::{hex::Hex, serde_as};

std::include!("coprocessor_node.v1.rs");

/// gRPC reflection binary. 
pub const FILE_DESCRIPTOR_SET: &[u8] = std::include_bytes!("descriptor.bin");
