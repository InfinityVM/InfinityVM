//! Coprocessor Node components.

pub mod cli;
/// Configuration structures for the coprocessor node, including remote ELF store settings.
pub mod config;
pub mod event;
pub mod gateway;
pub mod intake;
pub mod job_executor;
pub mod metrics;
pub mod node;
pub mod queue;
pub mod relayer;
pub mod elf_store;
pub mod server;
pub mod writer;

/// Default maximum bytes of DA allowed per job.
pub const MAX_DA_PER_JOB: usize =
    ivm_eip4844::SIMPLE_CODER_MAX_DATA_PER_BLOB * ivm_eip4844::MAX_BLOBS_PER_BLOCK;
