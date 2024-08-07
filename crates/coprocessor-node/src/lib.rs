//! The coprocessor node.

/// CLI scaffolding
pub mod cli;

/// gRPC Server
pub mod service;

pub mod relayer;

pub mod gateway;

/// Job processor
pub mod job_processor;

pub mod event;
