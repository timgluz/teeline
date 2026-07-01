use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use tower::{Layer, Service};

use crate::metrics::{HttpDurationLabels, HttpLabels, MetricsState};

#[derive(Clone)]
pub struct MetricsLayer {
    metrics: Arc<MetricsState>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<MetricsState>) -> Self {
        Self { metrics }
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsService<S>;

    fn layer(&self, inner: S) -> MetricsService<S> {
        MetricsService {
            inner,
            metrics: Arc::clone(&self.metrics),
        }
    }
}

#[derive(Clone)]
pub struct MetricsService<S> {
    inner: S,
    metrics: Arc<MetricsState>,
}

impl<S, B, RB> Service<Request<B>> for MetricsService<S>
where
    S: Service<Request<B>, Response = Response<RB>> + Clone + Send + 'static,
    S::Error: Send,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let metrics = Arc::clone(&self.metrics);
        let start = Instant::now();

        // MatchedPath is populated by axum's routing layer before this middleware runs
        // (Router::layer() applies middleware after routing, before the handler).
        // Falls back to raw URI path for unmatched routes — all our routes are fixed
        // paths with no path parameters so raw path is safe (no cardinality explosion).
        let path = req
            .extensions()
            .get::<MatchedPath>()
            .map(|m| m.as_str().to_owned())
            .unwrap_or_else(|| req.uri().path().to_owned());
        let method = req.method().as_str().to_owned();

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let resp = inner.call(req).await?;
            let elapsed = start.elapsed().as_secs_f64();
            let status = resp.status().as_u16().to_string();

            // One-liner guards: get_or_create() returns MappedRwLockReadGuard.
            // Never bind to a `let` before an await — the guard must be dropped immediately.
            metrics
                .http_requests_total
                .get_or_create(&HttpLabels {
                    method: method.clone(),
                    path: path.clone(),
                    status,
                })
                .inc();
            metrics
                .http_request_duration_seconds
                .get_or_create(&HttpDurationLabels { method, path })
                .observe(elapsed);

            Ok(resp)
        })
    }
}
