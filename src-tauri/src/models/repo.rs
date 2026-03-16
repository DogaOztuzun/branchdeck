use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub current_branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePreview {
    pub sanitized_name: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
    pub branch_exists: bool,
    pub path_exists: bool,
    pub worktree_exists: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
    pub has_worktree: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrackingInfo {
    pub ahead: usize,
    pub behind: usize,
    pub upstream_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub url: String,
    pub ci_status: Option<String>,
}
