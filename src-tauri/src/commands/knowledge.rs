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

#[cfg(feature = "knowledge")]
#[tauri::command]
pub async fn forget_knowledge(
    _knowledge: State<'_, Arc<KnowledgeService>>,
    _entry_id: u64,
) -> Result<(), AppError> {
    // TODO: Implement soft-delete via metadata flag once RVF supports update-in-place.
    // For now, entries cannot be deleted from the vector store.
    Err(AppError::Knowledge(
        "Forget is not yet implemented — vector store does not support deletion".to_string(),
    ))
}

#[cfg(feature = "sona")]
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn suggest_next(
    knowledge: State<'_, Arc<KnowledgeService>>,
    repo_path: String,
    context: String,
    top_k: Option<usize>,
) -> Result<Vec<crate::models::knowledge::Suggestion>, AppError> {
    if context.trim().is_empty() {
        return Err(AppError::Knowledge("context must not be empty".to_string()));
    }
    let k = top_k.unwrap_or(5).min(20);
    knowledge.suggest_next(&repo_path, &context, k).await
}
