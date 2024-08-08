//! The coprocessor node.

/// CLI scaffolding
pub mod cli;

/// gRPC Server
pub mod service;

pub mod relayer;

/// Job processor
pub mod job_processor;

pub mod event;

/// Metrics
pub mod metrics;
