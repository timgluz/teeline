use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use prometheus_client::encoding::text::encode;

use crate::AppState;

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut body = String::new();
    let _ = encode(&mut body, &state.metrics.registry);
    (
        [(
            header::CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )],
        body,
    )
}
