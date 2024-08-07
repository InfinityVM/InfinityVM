//! zkvm tracing util

use std::{env, str::FromStr};
use strum_macros::EnumString;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};

#[derive(EnumString, Debug, Default)]
#[strum(serialize_all = "lowercase")]
enum LogFormat {
    #[default]
    Text,
    Json,
}

/// zkvm tracing util to init logging
pub fn init_logging() -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    let rust_log_file = env::var("RUST_LOG_FILE").unwrap_or_default();
    let rust_log_dir = env::var("RUST_LOG_DIR").unwrap_or_else(|_| ".".to_string());

    let rust_log_format = env::var("RUST_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    let log_format = LogFormat::from_str(&rust_log_format).unwrap_or_default();

    let (writer, guard) = if !rust_log_file.is_empty() {
        let appender = RollingFileAppender::new(Rotation::NEVER, rust_log_dir, rust_log_file);
        tracing_appender::non_blocking(appender)
    } else {
        tracing_appender::non_blocking(std::io::stdout())
    };

    let subscriber_builder = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env());

    match log_format {
        LogFormat::Json => {
            subscriber_builder.json().init();
        }
        LogFormat::Text => {
            subscriber_builder.init();
        }
    }

    Ok(guard)
}
