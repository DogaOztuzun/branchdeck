use crate::error::AppError;
use crate::models::github::{PrFilter, PrSummary};
use crate::models::PrInfo;
use crate::services::github;
use git2::Repository;
use log::{debug, error};
use std::path::PathBuf;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn get_pr_status(
    repo_path: String,
    branch_name: String,
) -> Result<Option<PrInfo>, AppError> {
    // Extract owner/repo from git remote in a non-async block so non-Send
    // git2 types don't live across the await point.
    let parsed = {
        let repo = Repository::open(PathBuf::from(&repo_path)).map_err(|e| {
            error!("Failed to open repo for PR status: {e}");
            AppError::Git(e)
        })?;

        let remote = repo.find_remote("origin").map_err(|e| {
            debug!("No origin remote for {repo_path}: {e}");
            AppError::Git(e)
        })?;

        let remote_url = remote.url().unwrap_or("").to_string();
        github::parse_github_remote(&remote_url)
    };

    let Some((owner, repo_name)) = parsed else {
        debug!("Not a GitHub remote for {repo_path}");
        return Ok(None);
    };

    github::get_pr_for_branch(&owner, &repo_name, &branch_name).await
}

#[tauri::command]
pub async fn check_github_available() -> Result<bool, AppError> {
    match github::resolve_github_token().await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_open_prs(
    repo_path: String,
    filter: Option<PrFilter>,
) -> Result<Vec<PrSummary>, AppError> {
    github::list_open_prs(&repo_path, filter).await
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_all_open_prs(
    repo_paths: Vec<String>,
    filter: Option<PrFilter>,
) -> Result<Vec<PrSummary>, AppError> {
    github::list_all_open_prs(&repo_paths, filter).await
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn enrich_pr_summary(
    repo_path: String,
    mut pr: PrSummary,
) -> Result<PrSummary, AppError> {
    github::enrich_pr_summary(&repo_path, &mut pr).await?;
    Ok(pr)
}
