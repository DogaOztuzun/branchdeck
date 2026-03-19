//! Knowledge merge-on-delete and cross-repo promotion.

#[cfg(feature = "knowledge")]
use crate::error::AppError;
#[cfg(feature = "knowledge")]
use crate::models::knowledge::{KnowledgeType, MergeResult};
#[cfg(feature = "knowledge")]
use crate::services::knowledge::{repo_hash, KnowledgeService, StoreHandle};
#[cfg(feature = "knowledge")]
use log::{debug, error, info, warn};
#[cfg(feature = "knowledge")]
use std::collections::HashMap;
#[cfg(feature = "knowledge")]
use std::sync::Arc;
#[cfg(feature = "knowledge")]
use tokio::sync::RwLock;

#[cfg(feature = "knowledge")]
type RepoStoreMap = HashMap<String, Arc<RwLock<StoreHandle>>>;

#[cfg(feature = "knowledge")]
impl KnowledgeService {
    /// Merge or delete worktree knowledge when a worktree is removed.
    ///
    /// If `promote=true`: succeeded trajectories and explicit entries have their
    /// `worktree_id` cleared (promoted to repo-level). Failed trajectories are discarded.
    ///
    /// If `promote=false`: all vectors with the matching `worktree_id` are soft-deleted.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if the repo store is not open.
    pub async fn merge_worktree_knowledge(
        &self,
        repo_path: &str,
        worktree_id: &str,
        promote: bool,
    ) -> Result<MergeResult, AppError> {
        let hash = repo_hash(repo_path);

        let store_lock = self.get_repo_store(&hash).await.ok_or_else(|| {
            AppError::Knowledge(format!("Repo store not open for {repo_path:?}"))
        })?;

        let mut store = store_lock.write().await;

        // Find all entries with this worktree_id
        let matching_ids: Vec<u64> = store
            .entries
            .values()
            .filter(|e| e.worktree_id.as_deref() == Some(worktree_id))
            .map(|e| e.id)
            .collect();

        if matching_ids.is_empty() {
            debug!("No knowledge entries found for worktree {worktree_id:?}");
            return Ok(MergeResult {
                promoted: 0,
                discarded: 0,
                deleted: 0,
            });
        }

        if !promote {
            // Soft-delete all matching vectors
            let delete_count = matching_ids.len() as u64;
            if let Err(e) = store.store.delete(&matching_ids) {
                error!("Failed to delete worktree vectors: {e}");
            }
            // Remove from content index
            for id in &matching_ids {
                store.entries.remove(id);
            }
            info!(
                "Deleted {delete_count} knowledge entries for worktree {worktree_id:?}"
            );
            return Ok(MergeResult {
                promoted: 0,
                discarded: 0,
                deleted: delete_count,
            });
        }

        // Promote: succeeded trajectories + explicit entries → clear worktree_id
        let mut promoted = 0u64;
        let mut discarded = 0u64;

        let mut ids_to_discard = Vec::new();

        for id in &matching_ids {
            if let Some(entry) = store.entries.get_mut(id) {
                let should_promote = match entry.entry_type {
                    KnowledgeType::Trajectory => entry.metadata.quality_score > 0.5,
                    KnowledgeType::Explicit
                    | KnowledgeType::Commit
                    | KnowledgeType::ErrorResolution
                    | KnowledgeType::Pattern => true,
                };

                if should_promote {
                    entry.worktree_id = None;
                    promoted += 1;
                } else {
                    ids_to_discard.push(*id);
                    discarded += 1;
                }
            }
        }

        // Delete discarded vectors from RVF
        if !ids_to_discard.is_empty() {
            if let Err(e) = store.store.delete(&ids_to_discard) {
                error!("Failed to delete discarded worktree vectors: {e}");
            }
            for id in &ids_to_discard {
                store.entries.remove(id);
            }
        }

        // Rewrite content index to persist worktree_id changes
        if promoted > 0 {
            rewrite_content_index(&store);
        }

        info!(
            "Merge-on-delete for worktree {worktree_id:?}: promoted={promoted}, discarded={discarded}"
        );

        Ok(MergeResult {
            promoted,
            discarded,
            deleted: 0,
        })
    }

    /// Cross-repo pattern promotion: find patterns that appear in 2+ repos
    /// and promote them to global.rvf.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if promotion fails.
    pub async fn promote_cross_repo_patterns(
        &self,
        min_occurrences: usize,
    ) -> Result<u64, AppError> {
        // Collect entries from repos, then drop the lock before embedding
        let mut seen_content: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        {
            let repo_stores: tokio::sync::RwLockReadGuard<'_, RepoStoreMap> =
                self.repo_stores.read().await;

            if repo_stores.len() < min_occurrences {
                debug!(
                    "Only {} repos open, need at least {} for cross-repo promotion",
                    repo_stores.len(),
                    min_occurrences
                );
                return Ok(0);
            }

            for store_lock in repo_stores.values() {
                let store: tokio::sync::RwLockReadGuard<'_, StoreHandle> =
                    store_lock.read().await;
                for entry in store
                    .entries
                    .values()
                    .filter(|e| e.metadata.quality_score >= 0.8)
                    .take(50)
                {
                    *seen_content.entry(entry.content.clone()).or_insert(0) += 1;
                }
            }
        } // repo_stores lock dropped here — open_repo/close_repo won't be blocked

        let mut promoted = 0u64;

        // Pre-embed candidates outside the global store lock to avoid
        // blocking queries during ONNX inference
        let mut candidates: Vec<(String, Vec<f32>)> = Vec::new();
        {
            let global_lock = Arc::clone(self.global_store());
            let global_store = global_lock.read().await;

            for (content, count) in &seen_content {
                if *count >= min_occurrences {
                    let already_exists = global_store
                        .entries
                        .values()
                        .any(|e| e.content == *content);

                    if !already_exists {
                        if let Some(embedding) = self.embed_text(content).await {
                            candidates.push((content.clone(), embedding));
                        }
                    }
                }
            }
        }

        // Now ingest with a short write lock
        let global_lock = Arc::clone(self.global_store());
        let mut global_store = global_lock.write().await;

        for (content, embedding) in &candidates {
            let id = global_store.allocate_id();
            let entry = crate::models::knowledge::KnowledgeEntry {
                id,
                content: content.clone(),
                entry_type: KnowledgeType::Pattern,
                source: crate::models::knowledge::KnowledgeSource::EventBus,
                repo_hash: String::new(),
                worktree_id: None,
                metadata: crate::models::knowledge::KnowledgeMetadata {
                    session_id: None,
                    tool_names: Vec::new(),
                    file_paths: Vec::new(),
                    run_status: None,
                    cost_usd: None,
                    quality_score: 1.0,
                },
                created_at: crate::models::agent::now_ms(),
            };
            if let Err(e) = global_store.ingest(embedding, entry) {
                warn!("Failed to promote cross-repo pattern: {e}");
            } else {
                promoted += 1;
            }
        }

        if promoted > 0 {
            info!("Promoted {promoted} cross-repo patterns to global store");
        }

        Ok(promoted)
    }
}

/// Rewrite the entire content index JSONL from the in-memory entries.
/// Called after operations that modify entry metadata (e.g., merge-on-delete).
#[cfg(feature = "knowledge")]
fn rewrite_content_index(store: &StoreHandle) {
    use std::io::Write;

    let Ok(mut file) = std::fs::File::create(&store.entries_path) else {
        error!(
            "Failed to rewrite content index: {}",
            store.entries_path.display()
        );
        return;
    };

    for entry in store.entries.values() {
        let Ok(json) = serde_json::to_string(entry) else {
            continue;
        };
        if let Err(e) = writeln!(file, "{json}") {
            error!("Failed to write content index entry: {e}");
            return;
        }
    }

    if let Err(e) = file.sync_all() {
        warn!("Failed to fsync rewritten content index: {e}");
    }
}
