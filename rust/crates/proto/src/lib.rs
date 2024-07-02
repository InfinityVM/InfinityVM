//! The proto generated server, client, and messages

// We don't have control over tonic generated code so we ignore the
// lints it complains about
#![allow(clippy::all, clippy::missing_const_for_fn, unreachable_pub)]

tonic::include_proto!("zkvm_executor");
