use axum::extract::Query;
use axum::response::Json;
use branchdeck_core::models::project_config::{
    ProjectConfig, SetupStatus, TokenValidation, WorkflowOption,
};
use branchdeck_core::services::project_config;
use log::debug;
use serde::Deserialize;

use crate::error::ApiError;

#[derive(Deserialize)]
pub struct RepoQuery {
    pub repo_path: String,
}

pub async fn get_setup_status(
    Query(query): Query<RepoQuery>,
) -> Result<Json<SetupStatus>, ApiError> {
    debug!("Checking setup status for {:?}", query.repo_path);
    project_config::get_setup_status(&query.repo_path)
        .map(Json)
        .map_err(ApiError::from)
}

pub async fn validate_tokens() -> Result<Json<TokenValidation>, ApiError> {
    debug!("Validating token availability");
    let result = tokio::task::spawn_blocking(project_config::validate_tokens)
        .await
        .map_err(|e| {
            ApiError::from(branchdeck_core::error::AppError::Config(format!(
                "token validation task failed: {e}"
            )))
        })?;
    Ok(Json(result))
}

pub async fn list_workflows(Query(query): Query<RepoQuery>) -> Json<Vec<WorkflowOption>> {
    debug!("Listing available workflows for {:?}", query.repo_path);
    Json(project_config::list_available_workflows(&query.repo_path))
}

pub async fn save_config(Json(config): Json<ProjectConfig>) -> Result<Json<()>, ApiError> {
    debug!("Saving project config for {:?}", config.repo_path);
    project_config::save_project_config(&config)
        .map(Json)
        .map_err(ApiError::from)
}
