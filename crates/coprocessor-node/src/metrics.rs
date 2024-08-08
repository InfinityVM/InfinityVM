use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{self, CounterVec, Encoder, Opts, Registry, TextEncoder};
use std::{fmt::Debug, sync::Arc};

/// Custom prometheus metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    job_errors: CounterVec,
    relay_errors: CounterVec,
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
        registry.register(Box::new(job_errors.clone())).unwrap();
        registry.register(Box::new(relay_errors.clone())).unwrap();

        Self { job_errors, relay_errors }
    }

    /// Increment job errors counter
    pub fn incr_job_err(&self, label: &str) {
        self.job_errors.with_label_values(&[label]).inc();
    }
    /// Increment relayer errors counter
    pub fn incr_relay_err(&self, label: &str) {
        self.relay_errors.with_label_values(&[label]).inc();
    }
}

impl MetricServer {
    /// Return a new server instance
    pub fn new(registry: Arc<Registry>) -> Self {
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
