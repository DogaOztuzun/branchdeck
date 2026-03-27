use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::workflow::WorkflowDef;
use branchdeck_core::services::workflow::{self, WorkflowRegistry};

use crate::error::ApiError;
use crate::state::AppState;

/// Summary of a workflow for API responses (avoids exposing raw prompt).
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub trigger_kind: String,
    pub outcome_count: usize,
}

impl From<&WorkflowDef> for WorkflowSummary {
    fn from(def: &WorkflowDef) -> Self {
        Self {
            name: def.config.name.clone(),
            description: def.config.description.clone(),
            trigger_kind: def.config.tracker.kind.to_string(),
            outcome_count: def.config.outcomes.len(),
        }
    }
}

/// Detail view of a workflow for API responses.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDetail {
    pub config: branchdeck_core::models::workflow::WorkflowConfig,
    pub prompt: String,
}

impl From<&WorkflowDef> for WorkflowDetail {
    fn from(def: &WorkflowDef) -> Self {
        Self {
            config: def.config.clone(),
            prompt: def.prompt.clone(),
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/workflows",
    responses(
        (status = 200, description = "List all registered workflows", body = Vec<WorkflowSummary>)
    ),
    tag = "workflows"
)]
pub async fn list_workflows(State(state): State<AppState>) -> Json<Vec<WorkflowSummary>> {
    let search_dirs = workflow::default_search_dirs(&state.workspace_root.display().to_string());
    let registry = WorkflowRegistry::scan(&search_dirs);
    let summaries = registry.list_workflows().iter().map(|w| (*w).into()).collect();
    Json(summaries)
}

#[utoipa::path(
    get,
    path = "/api/workflows/{name}",
    params(
        ("name" = String, Path, description = "Workflow name")
    ),
    responses(
        (status = 200, description = "Workflow detail", body = WorkflowDetail),
        (status = 404, description = "Workflow not found", body = crate::error::ProblemDetails)
    ),
    tag = "workflows"
)]
pub async fn get_workflow(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<WorkflowDetail>, ApiError> {
    let search_dirs = workflow::default_search_dirs(&state.workspace_root.display().to_string());
    let registry = WorkflowRegistry::scan(&search_dirs);
    let def = registry.get_workflow(&name).ok_or_else(|| {
        branchdeck_core::error::AppError::Workflow(format!("workflow not found: {name}"))
    })?;
    Ok(Json(def.into()))
}
