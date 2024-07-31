//! The coprocessor node.

/// CLI scaffolding
pub mod cli;

/// gRPC Server
pub mod service;

pub mod relayer;

pub mod contracts;

#[cfg(test)]
pub mod test_utils;
