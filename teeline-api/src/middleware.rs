use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::extract::MatchedPath;
use axum::http::{HeaderMap, Request, Response};
use axum::response::IntoResponse;
use subtle::ConstantTimeEq;
use tower::{Layer, Service};

use crate::error::ApiError;
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

const AUTH_EXEMPT_PATH: &str = "/api/v1/health";

#[derive(Clone)]
pub struct AuthLayer {
    token: Arc<str>,
}

impl AuthLayer {
    pub fn new(token: impl Into<Arc<str>>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> AuthService<S> {
        AuthService {
            inner,
            token: Arc::clone(&self.token),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    token: Arc<str>,
}

impl<S, B> Service<Request<B>> for AuthService<S>
where
    S: Service<Request<B>, Response = Response<axum::body::Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send,
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
        // MatchedPath is populated by axum's routing layer before middleware
        // added via Router::layer() runs (same ordering MetricsService above
        // already relies on). The api sub-router only ever matches
        // "/api/v1/health" as the exempt path.
        let is_health = req
            .extensions()
            .get::<MatchedPath>()
            .is_some_and(|m| m.as_str() == AUTH_EXEMPT_PATH);

        if is_health || self.token_ok(req.headers()) {
            let clone = self.inner.clone();
            let mut inner = std::mem::replace(&mut self.inner, clone);
            return Box::pin(async move { inner.call(req).await });
        }

        Box::pin(async move { Ok(ApiError::Unauthorized.into_response()) })
    }
}

impl<S> AuthService<S> {
    fn token_ok(&self, headers: &HeaderMap) -> bool {
        extract_token(headers).is_some_and(|presented| {
            let expected = self.token.as_bytes();
            let presented = presented.as_bytes();
            // Constant-time comparison to avoid timing side-channels. Token
            // length is not itself secret, so a plain length check first is
            // fine; ct_eq handles the equal-length byte comparison.
            expected.len() == presented.len() && expected.ct_eq(presented).into()
        })
    }
}

/// Extracts the bearer/api-key token from either `Authorization: Bearer <token>`
/// or `X-Api-Key: <token>`, preferring `Authorization` if both are present.
fn extract_token(headers: &HeaderMap) -> Option<String> {
    if let Some(v) = headers.get(axum::http::header::AUTHORIZATION)
        && let Ok(s) = v.to_str()
        && let Some(token) = s.strip_prefix("Bearer ")
    {
        return Some(token.to_string());
    }
    headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}
