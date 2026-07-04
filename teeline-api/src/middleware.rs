use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant;

use axum::extract::MatchedPath;
use axum::http::{HeaderMap, Request, Response};
use axum::response::IntoResponse;
use subtle::ConstantTimeEq;
use tower::{Layer, Service};

use crate::clerk::ApiKeyVerifier;
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

/// Wraps `router` so every request must present a valid credential, except
/// `AUTH_EXEMPT_PATH`. A request is authorized if it presents either the
/// static break-glass `token` (operator credential, works even if Clerk is
/// unreachable) or a key that `verifier` confirms is a live, non-revoked,
/// non-expired Clerk-issued API key. Uses `route_layer` (not `layer`) so
/// requests that don't match any route on `router` fall through to axum's
/// normal 404 instead of getting a 401 — `.layer()` would also wrap the
/// router's fallback, turning every unmatched path into a 401 (axum's
/// `route_layer` docs call this exact scenario out).
pub fn require_auth(
    router: axum::Router<crate::AppState>,
    token: impl Into<Arc<str>>,
    verifier: Arc<dyn ApiKeyVerifier>,
) -> axum::Router<crate::AppState> {
    let token: Arc<str> = token.into();
    router.route_layer(axum::middleware::from_fn(
        move |matched_path: Option<MatchedPath>,
              headers: HeaderMap,
              request: axum::extract::Request,
              next: axum::middleware::Next| {
            let token = Arc::clone(&token);
            let verifier = Arc::clone(&verifier);
            async move {
                // MatchedPath is populated by axum's routing before route_layer
                // middleware runs. The api sub-router only ever matches
                // "/api/v1/health" as the exempt path.
                let is_health = matched_path.is_some_and(|m| m.as_str() == AUTH_EXEMPT_PATH);
                if is_health {
                    return next.run(request).await;
                }
                if token_matches(&token, &headers) {
                    tracing::info!("request authorized via static break-glass API_KEY");
                    return next.run(request).await;
                }
                if let Some(presented) = extract_token(&headers)
                    && let Some(verified) = verifier.verify(presented).await
                {
                    tracing::info!(subject = %verified.subject, "request authorized via Clerk API key");
                    return next.run(request).await;
                }
                ApiError::Unauthorized.into_response()
            }
        },
    ))
}

fn token_matches(token: &str, headers: &HeaderMap) -> bool {
    // Defense in depth: an empty configured token must never match, even
    // though the real fix is api_key() in main.rs never producing one.
    // Without this, an empty configured token would authenticate any
    // request presenting an empty (but present) credential.
    if token.is_empty() {
        return false;
    }
    extract_token(headers).is_some_and(|presented| {
        let expected = token.as_bytes();
        let presented = presented.as_bytes();
        // Constant-time comparison to avoid timing side-channels. Token
        // length is not itself secret, so a plain length check first is
        // fine; ct_eq handles the equal-length byte comparison.
        expected.len() == presented.len() && expected.ct_eq(presented).into()
    })
}

/// Extracts the bearer/api-key token from either `Authorization: Bearer <token>`
/// or `X-Api-Key: <token>`, preferring `Authorization` if both are present.
/// The `Bearer` scheme name is matched case-insensitively per RFC 7235 §2.1.
/// Borrows from `headers` rather than allocating — this runs on every
/// authenticated request.
fn extract_token(headers: &HeaderMap) -> Option<&str> {
    if let Some(v) = headers.get(axum::http::header::AUTHORIZATION)
        && let Ok(s) = v.to_str()
        && let Some((scheme, token)) = s.split_once(' ')
        && scheme.eq_ignore_ascii_case("bearer")
    {
        return Some(token);
    }
    headers.get("X-Api-Key").and_then(|v| v.to_str().ok())
}
