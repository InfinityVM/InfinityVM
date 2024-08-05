use prometheus::{CounterVec, Opts, Registry};

#[derive(Debug, Clone)]
pub struct Metrics {
    pub job_errors: CounterVec,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Self {
        let job_errors_opts = Opts::new("job_errors_total", "Total job processing errors");
        let job_errors = CounterVec::new(job_errors_opts, &["error_type"]).unwrap();
        registry.register(Box::new(job_errors.clone())).unwrap();

        Self { job_errors }
    }
}
