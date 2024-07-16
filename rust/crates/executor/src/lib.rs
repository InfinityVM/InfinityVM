//! zkvm execution gRPC server.

use alloy::hex;

pub mod cli;
pub mod db;
pub mod service;

/// Executor operators private key for development
pub const DEV_SECRET: [u8; 32] =
    hex!("2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6");
