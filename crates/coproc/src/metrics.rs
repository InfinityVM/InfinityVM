//! Prometheus metrics registry wrapper and server.

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{
    self, Counter, CounterVec, Encoder, Histogram, HistogramOpts, Opts, Registry, TextEncoder,
};
use std::{fmt::Debug, sync::Arc, time::Duration};

/// Custom prometheus metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    job_errors: CounterVec,
    relay_errors: CounterVec,
    relayed_total: Counter,
    job_exec_time: Histogram,
}

/// Metrics Server
#[derive(Debug, Default, Clone)]
pub struct MetricServer {
    registry: Arc<Registry>,
}

impl Metrics {
    /// Create a new metrics object
    pub fn new(registry: &Registry) -> Self {
        let job_errors_opts = Opts::new("job_errors_total", "Total job processing errors");
        let relay_errors_opts = Opts::new("relay_errors_total", "Total relay errors");

        let job_errors = CounterVec::new(job_errors_opts, &["error_type"]).unwrap();
        let relay_errors = CounterVec::new(relay_errors_opts, &["error_type"]).unwrap();
        let relayed_total = Counter::new("relayed_total", "Total number of jobs relayed").unwrap();
        let job_exec_time = Histogram::with_opts(
            HistogramOpts::new("job_exec_time", "Total time for a single job to execute").buckets(
                // From 562.5ms to 16s, polynomial
                vec![
                    0.5625, 1., 1.5625, 2.25, 3.0625, 4., 5.0625, 6.25, 7.5625, 9.,
                    10.5625, 12.25, 14.0625, 16.,
                ],
            ),
        )
        .unwrap();

        registry.register(Box::new(job_errors.clone())).unwrap();
        registry.register(Box::new(relay_errors.clone())).unwrap();
        registry.register(Box::new(relayed_total.clone())).unwrap();
        registry.register(Box::new(job_exec_time.clone())).unwrap();

        Self { job_errors, relay_errors, relayed_total, job_exec_time }
    }

    /// Increment job errors counter
    pub fn incr_job_err(&self, label: &str) {
        self.job_errors.with_label_values(&[label]).inc();
    }
    /// Increment relayer errors counter
    pub fn incr_relay_err(&self, label: &str) {
        self.relay_errors.with_label_values(&[label]).inc();
    }
    /// Increment counter for total number of jobs relayed
    pub fn incr_relayed_total(&self) {
        self.relayed_total.inc();
    }
    /// Record a time measurement of a single job execution.
    pub fn observe_job_exec_time(&self, duration: Duration) {
        self.job_exec_time.observe(duration.as_secs_f64());
    }
}

impl MetricServer {
    /// Return a new server instance
    pub const fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }

    /// Serve metrics
    pub async fn serve(&self, addr: &str) -> Result<(), std::io::Error> {
        let registry = Arc::clone(&self.registry);

        let router =
            Router::new().route("/metrics", get(Self::handle_metrics)).with_state(registry);

        let addr: std::net::SocketAddr = addr.parse().unwrap();
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router).await
    }

    /// Metrics path
    async fn handle_metrics(State(registry): State<Arc<Registry>>) -> impl IntoResponse {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();
        let metric_families = registry.gather();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        let metrics = String::from_utf8(buffer).unwrap();
        (StatusCode::OK, metrics).into_response()
    }
}
