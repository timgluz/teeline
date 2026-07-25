use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

pub type ApiResult<T> = Result<T, ApiError>;

pub enum ApiError {
    BadRequest(String),
    Internal(String),
    Unauthorized,
}

impl ApiError {
    /// Maps a service-layer error string to the right HTTP status: solver
    /// panics (surfaced from `tokio::task::spawn_blocking`'s `JoinError` as
    /// `format!("task panic: {e}")`) are a server-side fault, everything else
    /// (validation failures, unknown solver names) is a client error. Shared
    /// by `routes::solve::solve` and `routes::pipeline::pipeline`, which both
    /// call solver-service methods with this exact error convention.
    pub fn from_solver_error(e: String) -> Self {
        if e.starts_with("task panic:") {
            ApiError::Internal(e)
        } else {
            ApiError::BadRequest(e)
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // RFC 7235 §3.1: a 401 response must include WWW-Authenticate
        // naming the scheme(s) this API accepts.
        let www_authenticate = matches!(self, ApiError::Unauthorized)
            .then(|| HeaderValue::from_static(r#"Bearer, ApiKey realm="teeline-api""#));

        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
        };
        let body = Json(serde_json::json!({ "error": message }));
        let mut response = (status, body).into_response();
        if let Some(value) = www_authenticate {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, value);
        }
        response
    }
}
