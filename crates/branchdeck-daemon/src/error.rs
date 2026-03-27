use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Wrapper around `branchdeck_core::error::AppError` that implements
/// Axum's `IntoResponse` for RFC 7807 Problem Details responses.
pub struct ApiError(branchdeck_core::error::AppError);

#[derive(Serialize)]
struct ProblemDetail {
    r#type: &'static str,
    title: String,
    status: u16,
    detail: String,
}

impl From<branchdeck_core::error::AppError> for ApiError {
    fn from(err: branchdeck_core::error::AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        use branchdeck_core::error::AppError;

        let (status, title) = match &self.0 {
            AppError::RunError(_) => (StatusCode::CONFLICT, "Run Error"),
            AppError::TaskNotFound(_) => (StatusCode::NOT_FOUND, "Task Not Found"),
            AppError::SidecarError(_) => (StatusCode::BAD_GATEWAY, "Sidecar Error"),
            AppError::Sat(_) => (StatusCode::UNPROCESSABLE_ENTITY, "SAT Error"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error"),
        };

        let body = ProblemDetail {
            r#type: "about:blank",
            title: title.to_owned(),
            status: status.as_u16(),
            detail: self.0.to_string(),
        };

        (
            status,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/problem+json",
            )],
            axum::Json(body),
        )
            .into_response()
    }
}
