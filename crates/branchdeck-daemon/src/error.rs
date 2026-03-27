use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
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
                (StatusCode::NOT_FOUND, "Not Found")
            }
            AppError::TaskAlreadyExists(_) => (StatusCode::CONFLICT, "Conflict"),
            AppError::Config(_) | AppError::TaskParseError(_) | AppError::Workflow(_) => {
                (StatusCode::BAD_REQUEST, "Bad Request")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
        };

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
        StatusCode::NOT_FOUND => "not-found",
        StatusCode::CONFLICT => "conflict",
        StatusCode::BAD_REQUEST => "bad-request",
        _ => "internal-error",
    }
}
