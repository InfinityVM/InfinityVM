//! The coprocessor node.

/// CLI scaffolding
pub mod cli;

/// gRPC Server
pub mod service;

pub mod result_writer;

pub mod contracts;

#[cfg(test)]
pub mod test_utils;
