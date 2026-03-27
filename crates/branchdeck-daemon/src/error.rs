use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use branchdeck_core::error::AppError;
use log::error;
use serde::Serialize;
use utoipa::ToSchema;

/// RFC 7807 Problem Details response body.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProblemDetails {
    /// URI reference identifying the problem type.
    #[serde(rename = "type")]
    pub problem_type: String,
    /// Short human-readable summary.
    pub title: String,
    /// HTTP status code.
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Wrapper around `AppError` that implements `IntoResponse` with RFC 7807 format.
pub struct ApiError(pub branchdeck_core::error::AppError);

impl From<branchdeck_core::error::AppError> for ApiError {
    fn from(err: branchdeck_core::error::AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        use branchdeck_core::error::AppError;

        let (status, title) = match &self.0 {
            AppError::TaskNotFound(_) | AppError::RunError(_) => {
                // Distinguish "not implemented" stubs from actual not-found errors
                let msg = self.0.to_string();
                if msg.contains("Not implemented") {
                    (StatusCode::NOT_IMPLEMENTED, "Not Implemented")
                } else {
                    (StatusCode::NOT_FOUND, "Not Found")
                }
            }
            AppError::TaskAlreadyExists(_) => (StatusCode::CONFLICT, "Conflict"),
            AppError::Config(_) | AppError::TaskParseError(_) | AppError::Workflow(_) => {
                (StatusCode::BAD_REQUEST, "Bad Request")
            }
            AppError::SidecarError(_) => (StatusCode::BAD_GATEWAY, "Sidecar Error"),
            AppError::Sat(_) => (StatusCode::UNPROCESSABLE_ENTITY, "SAT Error"),
            _ => {
                // Inspect upstream errors to surface HTTP semantics instead of
                // collapsing everything to 500.
                let upstream = classify_upstream_error(&self.0);
                (
                    upstream,
                    upstream.canonical_reason().unwrap_or("Internal Server Error"),
                )
            }
        };

        if status.is_server_error() {
            error!("API error {status}: {}", self.0);
        }

        let problem = ProblemDetails {
            problem_type: format!("https://branchdeck.dev/problems/{}", slug_from_status(status)),
            title: title.to_string(),
            status: status.as_u16(),
            detail: Some(self.0.to_string()),
        };

        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/problem+json",
            )],
            axum::Json(problem),
        )
            .into_response()
    }
}

fn slug_from_status(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not-found",
        StatusCode::NOT_IMPLEMENTED => "not-implemented",
        StatusCode::CONFLICT => "conflict",
        StatusCode::BAD_REQUEST => "bad-request",
        StatusCode::BAD_GATEWAY => "sidecar-error",
        StatusCode::UNPROCESSABLE_ENTITY => "sat-error",
        StatusCode::TOO_MANY_REQUESTS => "rate-limited",
        _ => "internal-error",
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
