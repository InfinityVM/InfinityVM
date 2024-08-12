//! zkvm tracing util

use dotenv::dotenv;
use std::{env, fmt::Debug, io::stdout, str::FromStr};
use strum_macros::EnumString;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(EnumString, Debug, Default)]
#[strum(serialize_all = "lowercase")]
enum LogFormat {
    #[default]
    Text,
    Json,
}

/// zkvm tracing util to init logging
pub fn init_logging() -> Result<Vec<WorkerGuard>, Box<dyn std::error::Error>> {
    dotenv().ok();

    let rust_log_file = env::var("RUST_LOG_FILE").unwrap_or_default();
    let rust_log_dir = env::var("RUST_LOG_DIR").unwrap_or_else(|_| ".".to_string());

    let rust_log_format = env::var("RUST_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    let log_format = LogFormat::from_str(&rust_log_format).unwrap_or_default();

    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(stdout());
    let mut guards = vec![stdout_guard];

    let subscriber =
        tracing_subscriber::registry().with(tracing_subscriber::EnvFilter::from_default_env());

    if !rust_log_file.is_empty() {
        let appender = RollingFileAppender::new(Rotation::NEVER, rust_log_dir, rust_log_file);
        let (file_writer, file_guard) = tracing_appender::non_blocking(appender);
        guards.push(file_guard);

        match log_format {
            LogFormat::Json => {
                let stdout_layer = fmt::layer().with_writer(stdout_writer).json();
                let file_layer = fmt::layer().with_writer(file_writer).json();
                subscriber.with(stdout_layer).with(file_layer).init();
            }
            LogFormat::Text => {
                let file_layer = fmt::layer().with_writer(file_writer);
                subscriber.with(fmt::layer().with_writer(stdout_writer)).with(file_layer).init();
            }
        }
    } else {
        match log_format {
            LogFormat::Json => {
                let stdout_layer = fmt::layer().with_writer(stdout_writer).json();
                subscriber.with(stdout_layer).init();
            }
            LogFormat::Text => {
                subscriber.with(fmt::layer().with_writer(stdout_writer)).init();
            }
        }
    };

    Ok(guards)
}
