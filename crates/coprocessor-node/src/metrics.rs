use prometheus::{CounterVec, Opts, Registry};

/// Custom prometheus metrics
#[derive(Debug, Clone)]
pub struct Metrics {
    job_errors: CounterVec,
    relay_errors: CounterVec,
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
