//! The coprocessor node.

/// CLI scaffolding
pub mod cli;

/// gRPC Server
pub mod service;

pub mod relayer;

/// REST gRPC gateway
pub mod gateway;

/// Job processor
pub mod job_processor;

pub mod event;

/// Metrics
pub mod metrics;

pub mod node;
