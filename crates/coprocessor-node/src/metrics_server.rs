use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Router};
use prometheus::{self, Encoder, Registry, TextEncoder};
use std::{fmt::Debug, sync::Arc};

/// Metrics Server
#[derive(Debug, Default, Clone)]
pub struct MetricServer {
    registry: Arc<Registry>,
}

impl MetricServer {
    /// Return a new server instance
    pub fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }

    /// Serve metrics
    pub async fn serve(&self, addr: &str) -> Result<(), std::io::Error> {
        let registry = Arc::clone(&self.registry);
        let router = Router::new()
            .route("/metrics", get(Self::handle_metrics))
            .route("/root", get(Self::root))
            .with_state(registry);

        let addr: std::net::SocketAddr = addr.parse().unwrap();
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, router).await
    }

    /// Root path
    async fn root() -> &'static str {
        "Visit /metrics"
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
