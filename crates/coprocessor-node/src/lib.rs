//! Coprocessor Node components.

pub mod cli;
pub mod event;
pub mod gateway;
pub mod intake;
pub mod job_processor;
pub mod metrics;
pub mod node;
pub mod relayer;
pub mod server;

/// Default maximum bytes of DA allowed per job.
pub const MAX_DA_PER_JOB: usize =
    eip4844::SIMPLE_CODER_MAX_DATA_PER_BLOB * eip4844::MAX_BLOBS_PER_BLOCK;
