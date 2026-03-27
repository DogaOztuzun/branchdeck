use axum::response::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub struct HealthResponse {
    pub service: &'static str,
    pub version: &'static str,
    pub pid: u32,
    pub workspace_root: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Daemon health check", body = HealthResponse)
    ),
    tag = "health"
)]
pub async fn health(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "branchdeck-daemon",
        version: env!("CARGO_PKG_VERSION"),
        pid: std::process::id(),
        workspace_root: state.workspace_root.display().to_string(),
    })
}
