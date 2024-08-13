//! zkvm tracing util

use dotenv::dotenv;
use std::{env, fmt::Debug, io::stdout, str::FromStr};
use strum_macros::EnumString;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

#[derive(EnumString, Debug, Default)]
#[strum(serialize_all = "lowercase")]
enum LogFormat {
    #[default]
    Text,
    Json,
}

/// A boxed layer for tracing
pub type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync>;

/// zkvm tracing util to init logging
pub fn init_logging() -> Result<Vec<WorkerGuard>, Box<dyn std::error::Error>> {
    dotenv().ok();

    let rust_log_file = env::var("RUST_LOG_FILE").unwrap_or_default();
    let rust_log_dir = env::var("RUST_LOG_DIR").unwrap_or_else(|_| ".".to_string());
    let rust_log_format = env::var("RUST_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());

    let log_format = LogFormat::from_str(&rust_log_format).unwrap_or_default();
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(stdout());

    let mut guards = vec![stdout_guard];
    let mut layers: Vec<BoxedLayer<Registry>> = vec![apply_layer_format(&log_format, stdout_writer)];

    if !rust_log_file.is_empty() {
        let appender = RollingFileAppender::new(Rotation::NEVER, rust_log_dir, rust_log_file);
        let (file_writer, file_guard) = tracing_appender::non_blocking(appender);
        guards.push(file_guard);
        layers.push(apply_layer_format(&log_format, file_writer));
    }

    tracing_subscriber::registry().with(layers).try_init()?;

    Ok(guards)
}

fn apply_layer_format(log_format: &LogFormat, writer: NonBlocking) -> BoxedLayer<Registry> {
    match log_format {
        LogFormat::Json => fmt::layer()
            .json()
            .with_writer(writer)
            .with_filter(tracing_subscriber::EnvFilter::from_default_env())
            .boxed(),
        LogFormat::Text => fmt::layer()
            .with_writer(writer)
            .with_filter(tracing_subscriber::EnvFilter::from_default_env())
            .boxed(),
    }
}
