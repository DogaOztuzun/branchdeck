//! Tauri IPC commands for the knowledge service.

use crate::error::AppError;
#[cfg(feature = "knowledge")]
use crate::models::knowledge::{KnowledgeStats, KnowledgeType, QueryResult};
#[cfg(feature = "knowledge")]
use crate::services::knowledge::KnowledgeService;
#[cfg(feature = "knowledge")]
use std::sync::Arc;
#[cfg(feature = "knowledge")]
use tauri::State;

#[cfg(feature = "knowledge")]
#[tauri::command]
pub async fn query_knowledge(
    knowledge: State<'_, Arc<KnowledgeService>>,
    repo_path: String,
    worktree_id: Option<String>,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<QueryResult>, AppError> {
    let k = top_k.unwrap_or(5).min(100);
    knowledge
        .query(&repo_path, worktree_id.as_deref(), &query, k)
        .await
}

#[cfg(feature = "knowledge")]
#[tauri::command]
pub async fn ingest_knowledge(
    knowledge: State<'_, Arc<KnowledgeService>>,
    repo_path: String,
    worktree_id: Option<String>,
    content: String,
    entry_type: String,
) -> Result<u64, AppError> {
    let kt = match entry_type.as_str() {
        "commit" => KnowledgeType::Commit,
        "error_resolution" => KnowledgeType::ErrorResolution,
        "pattern" => KnowledgeType::Pattern,
        _ => KnowledgeType::Explicit,
    };

    knowledge
        .ingest_explicit(&repo_path, worktree_id.as_deref(), &content, kt)
        .await
}

#[cfg(feature = "knowledge")]
#[tauri::command]
pub async fn get_knowledge_stats(
    knowledge: State<'_, Arc<KnowledgeService>>,
    repo_path: String,
) -> Result<KnowledgeStats, AppError> {
    knowledge.get_stats(&repo_path).await
}
