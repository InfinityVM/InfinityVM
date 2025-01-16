//! zkvm tracing util

use dotenvy::dotenv;
use std::{env, fmt::Debug, io::stdout, str::FromStr};
use strum::EnumString;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    filter::LevelFilter, fmt, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};

#[derive(EnumString, Debug, Default)]
#[strum(serialize_all = "lowercase")]
enum LogFormat {
    #[default]
    Text,
    Json,
}

/// A boxed layer for tracing
pub type BoxedLayer<S> = Box<dyn Layer<S> + Send + Sync>;

/// Initialize logging.
///
/// By default this will initialize INFO text to stdout.
///
/// Env var options:
/// - `COPROC_LOG_FILE` - file name to write logs to. If empty, will not write logs to file.
/// - `COPROC_LOG_DIR` - directory to write logs to. If empty will write logs to current directory.
/// - `COPROC_LOG_FORMAT_FILE` - logging format for file target. Defaults to `json`. One of json,
///   text.
/// - `COPROC_LOG_FORMAT_STDOUT` - logging format for stdout target. Defaults to `text`. One of
///   json, text.
pub fn init_logging() -> eyre::Result<Vec<WorkerGuard>> {
    dotenv().ok();

    let env_log_file = env::var("COPROC_LOG_FILE").unwrap_or_default();
    let env_log_dir = env::var("COPROC_LOG_DIR").unwrap_or_else(|_| ".".to_string());
    let env_log_format_file =
        env::var("COPROC_LOG_FORMAT_FILE").unwrap_or_else(|_| "json".to_string());
    let env_log_format_stdout =
        env::var("COPROC_LOG_FORMAT_STDOUT").unwrap_or_else(|_| "text".to_string());

    let log_format_file = LogFormat::from_str(&env_log_format_file).unwrap_or_default();
    let log_format_stdout = LogFormat::from_str(&env_log_format_stdout).unwrap_or_default();

    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(stdout());

    let mut guards = vec![stdout_guard];
    let mut layers: Vec<BoxedLayer<Registry>> =
        vec![apply_layer_format(&log_format_stdout, stdout_writer)];

    if !env_log_file.is_empty() {
        let appender = RollingFileAppender::new(Rotation::NEVER, &env_log_dir, &env_log_file);
        let (file_writer, file_guard) = tracing_appender::non_blocking(appender);
        guards.push(file_guard);
        layers.push(apply_layer_format(&log_format_file, file_writer));
    }

    tracing_subscriber::registry().with(layers).try_init()?;

    tracing::info!(
        COPROC_LOG_FILE = env_log_file,
        COPROC_LOG_DIR = env_log_dir,
        COPROC_LOG_FORMAT_FILE = env_log_format_file,
        COPROC_LOG_FORMAT_STDOUT = env_log_format_stdout,
        RUST_LOG = env::var("RUST_LOG").unwrap_or_default(),
        "Logging options configured via env vars: "
    );

    Ok(guards)
}

fn apply_layer_format(log_format: &LogFormat, writer: NonBlocking) -> BoxedLayer<Registry> {
    match log_format {
        LogFormat::Json => fmt::layer()
            .with_span_events(FmtSpan::CLOSE)
            .json()
            .with_writer(writer)
            .with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .boxed(),
        LogFormat::Text => fmt::layer()
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(writer)
            .with_filter(
                EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .from_env_lossy(),
            )
            .boxed(),
    }
}
