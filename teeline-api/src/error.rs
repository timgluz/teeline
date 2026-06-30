use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type ApiResult<T> = Result<T, ApiError>;

pub enum ApiError {
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        let body = Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}
