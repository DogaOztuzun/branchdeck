use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use branchdeck_core::error::AppError;
use log::error;

/// Newtype wrapper for RFC 7807 Problem Details responses.
#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            AppError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            AppError::TaskAlreadyExists(_) => StatusCode::CONFLICT,
            AppError::Config(_) => StatusCode::BAD_REQUEST,
            _ => classify_upstream_error(&self.0),
        };

        if status.is_server_error() {
            error!("API error {status}: {}", self.0);
        }

        let body = serde_json::json!({
            "type": "about:blank",
            "title": status.canonical_reason().unwrap_or("Error"),
            "status": status.as_u16(),
            "detail": self.0.to_string(),
        });

        let headers = [(axum::http::header::CONTENT_TYPE, "application/problem+json")];
        (status, headers, axum::Json(body)).into_response()
    }
}

/// Inspect the error message string to surface upstream HTTP semantics
/// instead of collapsing everything to 500. Prevents the frontend retry
/// logic (which retries 5xx) from amplifying GitHub rate limits.
fn classify_upstream_error(err: &AppError) -> StatusCode {
    let msg = err.to_string().to_lowercase();

    if msg.contains("rate limit") || msg.contains("429") {
        return StatusCode::TOO_MANY_REQUESTS;
    }
    if msg.contains("403") || msg.contains("forbidden") {
        return StatusCode::FORBIDDEN;
    }

    StatusCode::INTERNAL_SERVER_ERROR
}
