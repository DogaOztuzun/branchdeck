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
///
/// Uses word-boundary matching via `contains_http_status` to avoid
/// false positives (e.g., port 50300 matching "503").
fn classify_upstream_error(err: &AppError) -> StatusCode {
    let msg = err.to_string().to_lowercase();

    if msg.contains("rate limit") || branchdeck_core::util::contains_http_status(&msg, "429") {
        return StatusCode::TOO_MANY_REQUESTS;
    }
    if branchdeck_core::util::contains_http_status(&msg, "403") || msg.contains("forbidden") {
        return StatusCode::FORBIDDEN;
    }

    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn classify_rate_limit_429() {
        let err = AppError::GitHub("rate limit exceeded (429)".to_string());
        assert_eq!(classify_upstream_error(&err), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn classify_rate_limit_text() {
        let err = AppError::GitHub("API rate limit exceeded".to_string());
        assert_eq!(classify_upstream_error(&err), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn classify_forbidden() {
        let err = AppError::GitHub("403 Forbidden".to_string());
        assert_eq!(classify_upstream_error(&err), StatusCode::FORBIDDEN);
    }

    #[test]
    fn classify_generic_error_is_500() {
        let err = AppError::GitHub("something went wrong".to_string());
        assert_eq!(
            classify_upstream_error(&err),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn classify_port_50300_not_matched_as_503() {
        let err = AppError::GitHub("connect to port 50300 failed".to_string());
        assert_eq!(
            classify_upstream_error(&err),
            StatusCode::INTERNAL_SERVER_ERROR,
            "port 50300 should not be classified as a 503 server error"
        );
    }

    #[test]
    fn classify_pr_4290_not_matched_as_429() {
        let err = AppError::GitHub("PR #4290 not found".to_string());
        assert_eq!(
            classify_upstream_error(&err),
            StatusCode::INTERNAL_SERVER_ERROR,
            "PR #4290 should not be classified as rate limit"
        );
    }
}
