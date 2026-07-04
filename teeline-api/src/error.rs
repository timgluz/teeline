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
