use axum::extract::State;
use axum::response::Json;
use branchdeck_core::models::{RepoInfo, WorktreeInfo};
use branchdeck_core::services::git;

use crate::error::ApiError;
use crate::state::AppState;

/// Repository info with its worktrees.
#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RepoDetail {
    pub repo: RepoInfo,
    pub worktrees: Vec<WorktreeInfo>,
}

#[utoipa::path(
    get,
    path = "/api/repos",
    responses(
        (status = 200, description = "Repository info with worktrees", body = RepoDetail),
        (status = 500, description = "Git error", body = crate::error::ProblemDetails)
    ),
    tag = "repos"
)]
pub async fn get_repo(State(state): State<AppState>) -> Result<Json<RepoDetail>, ApiError> {
    let repo = git::validate_repo(&state.workspace_root)?;
    let worktrees = git::list_worktrees(&state.workspace_root)?;
    Ok(Json(RepoDetail { repo, worktrees }))
}
