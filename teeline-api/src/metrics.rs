use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HttpLabels {
    pub method: String,
    pub path: String,
    pub status: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct HttpDurationLabels {
    pub method: String,
    pub path: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct SolverLabels {
    pub solver: String,
    pub status: String,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct SolverDurationLabels {
    pub solver: String,
}

pub struct MetricsState {
    pub http_requests_total: Family<HttpLabels, Counter>,
    pub http_request_duration_seconds: Family<HttpDurationLabels, Histogram>,
    pub solver_requests_total: Family<SolverLabels, Counter>,
    pub solver_duration_seconds: Family<SolverDurationLabels, Histogram>,
    pub registry: Registry,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsState {
    pub fn new() -> Self {
        let http_requests_total = Family::<HttpLabels, Counter>::default();
        let http_request_duration_seconds =
            Family::<HttpDurationLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(
                    [
                        0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.512,
                        1.024, 2.048,
                    ]
                    .into_iter(),
                )
            });
        let solver_requests_total = Family::<SolverLabels, Counter>::default();
        let solver_duration_seconds =
            Family::<SolverDurationLabels, Histogram>::new_with_constructor(|| {
                Histogram::new([0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0].into_iter())
            });

        let mut registry = Registry::default();
        registry.register(
            "http_requests",
            "Total HTTP requests",
            http_requests_total.clone(),
        );
        registry.register(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
            http_request_duration_seconds.clone(),
        );
        registry.register(
            "teeline_solver_requests",
            "Total solver requests",
            solver_requests_total.clone(),
        );
        registry.register(
            "teeline_solver_duration_seconds",
            "Solver duration in seconds",
            solver_duration_seconds.clone(),
        );

        Self {
            http_requests_total,
            http_request_duration_seconds,
            solver_requests_total,
            solver_duration_seconds,
            registry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus_client::encoding::text::encode;

    #[test]
    fn new_registry_encodes_all_metric_names() {
        let m = MetricsState::new();

        // Increment each family once so histogram suffixes appear in output
        m.http_requests_total
            .get_or_create(&HttpLabels {
                method: "GET".into(),
                path: "/test".into(),
                status: "200".into(),
            })
            .inc();
        m.http_request_duration_seconds
            .get_or_create(&HttpDurationLabels {
                method: "GET".into(),
                path: "/test".into(),
            })
            .observe(0.001);
        m.solver_requests_total
            .get_or_create(&SolverLabels {
                solver: "nn".into(),
                status: "success".into(),
            })
            .inc();
        m.solver_duration_seconds
            .get_or_create(&SolverDurationLabels {
                solver: "nn".into(),
            })
            .observe(0.5);

        let mut out = String::new();
        encode(&mut out, &m.registry).expect("encode must not fail");

        assert!(
            out.contains("http_requests_total"),
            "missing http_requests_total"
        );
        assert!(
            out.contains("http_request_duration_seconds_bucket"),
            "missing http_request_duration_seconds_bucket"
        );
        assert!(
            out.contains("teeline_solver_requests_total"),
            "missing teeline_solver_requests_total"
        );
        assert!(
            out.contains("teeline_solver_duration_seconds_bucket"),
            "missing teeline_solver_duration_seconds_bucket"
        );
    }
}
