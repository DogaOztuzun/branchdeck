//! Knowledge query — cascading vector search across worktree → repo → global.

#[cfg(feature = "knowledge")]
use crate::error::AppError;
#[cfg(feature = "knowledge")]
use crate::models::knowledge::{KnowledgeStats, KnowledgeType, QueryResult};
#[cfg(feature = "knowledge")]
use crate::services::knowledge::{field_ids, repo_hash, KnowledgeService, StoreHandle};
#[cfg(feature = "knowledge")]
use log::{debug, error};
#[cfg(feature = "knowledge")]
use rvf_runtime::{FilterExpr, QueryOptions};
#[cfg(feature = "knowledge")]
use std::collections::HashSet;
#[cfg(feature = "knowledge")]
use std::sync::Arc;
#[cfg(feature = "knowledge")]
use tokio::sync::RwLock;

#[cfg(feature = "knowledge")]
impl KnowledgeService {
    /// Cascading query: worktree-filtered → repo-level → global.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if embedding or query fails.
    pub async fn query(
        &self,
        repo_path: &str,
        worktree_id: Option<&str>,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>, AppError> {
        // Ensure embedder and embed the query
        if !self.ensure_embedder().await {
            return Err(AppError::Knowledge(
                "ONNX embedding model not available".to_string(),
            ));
        }

        let embedding = self.embed_text(query_text).await.ok_or_else(|| {
            AppError::Knowledge("Failed to embed query text".to_string())
        })?;

        let hash = repo_hash(repo_path);
        let mut all_results: Vec<QueryResult> = Vec::new();
        let mut seen_ids: HashSet<u64> = HashSet::new();

        // Tier 1: worktree-filtered query on repo store
        if let Some(wt_id) = worktree_id {
            if let Some(store_lock) = self.get_repo_store(&hash).await {
                let results = query_store_filtered(
                    &store_lock,
                    &embedding,
                    top_k,
                    Some(wt_id),
                )
                .await;
                for r in results {
                    if seen_ids.insert(r.id) {
                        all_results.push(r);
                    }
                }
            }
        }

        // Tier 2: repo-level query (no worktree filter)
        if let Some(store_lock) = self.get_repo_store(&hash).await {
            let results =
                query_store_filtered(&store_lock, &embedding, top_k, None).await;
            for r in results {
                if seen_ids.insert(r.id) {
                    all_results.push(r);
                }
            }
        } else {
            debug!("No repo store open for {repo_path:?}, skipping repo tier");
        }

        // Tier 3: global store
        let global_results =
            query_store_filtered(self.global_store(), &embedding, top_k, None).await;
        for r in global_results {
            if seen_ids.insert(r.id) {
                all_results.push(r);
            }
        }

        // Re-rank by distance and take top_k
        all_results.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        all_results.truncate(top_k);

        debug!(
            "Knowledge query returned {} results for {:?}",
            all_results.len(),
            query_text
        );

        Ok(all_results)
    }

    /// Get knowledge stats for a repo.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if the repo store is not open.
    pub async fn get_stats(&self, repo_path: &str) -> Result<KnowledgeStats, AppError> {
        let hash = repo_hash(repo_path);
        let store_lock = self
            .get_repo_store(&hash)
            .await
            .unwrap_or_else(|| Arc::clone(self.global_store()));

        let store = store_lock.read().await;
        Ok(compute_stats(&store))
    }
}

/// Query a single store with optional worktree filter.
#[cfg(feature = "knowledge")]
async fn query_store_filtered(
    store_lock: &Arc<RwLock<StoreHandle>>,
    embedding: &[f32],
    top_k: usize,
    worktree_id: Option<&str>,
) -> Vec<QueryResult> {
    let store = store_lock.read().await;

    let filter = worktree_id.map(|wt_id| {
        FilterExpr::Eq(
            field_ids::WORKTREE_ID,
            rvf_runtime::filter::FilterValue::String(wt_id.to_string()),
        )
    });

    let options = QueryOptions {
        filter,
        ..QueryOptions::default()
    };

    match store.store.query(embedding, top_k, &options) {
        Ok(results) => results
            .into_iter()
            .filter_map(|sr| {
                store.entries.get(&sr.id).map(|entry| QueryResult {
                    id: sr.id,
                    content: entry.content.clone(),
                    entry_type: entry.entry_type.clone(),
                    distance: sr.distance,
                    metadata: entry.metadata.clone(),
                })
            })
            .collect(),
        Err(e) => {
            error!("RVF query failed: {e}");
            Vec::new()
        }
    }
}

/// Compute stats from a store handle.
#[cfg(feature = "knowledge")]
fn compute_stats(store: &StoreHandle) -> KnowledgeStats {
    let total_entries = store.entries.len() as u64;
    let mut trajectory_count = 0u64;
    let mut explicit_count = 0u64;
    let mut quality_sum = 0.0f32;
    let mut last_ingested: Option<u64> = None;

    for entry in store.entries.values() {
        match entry.entry_type {
            KnowledgeType::Trajectory => trajectory_count += 1,
            KnowledgeType::Explicit => explicit_count += 1,
            _ => {}
        }
        quality_sum += entry.metadata.quality_score;
        match last_ingested {
            Some(ts) if entry.created_at > ts => last_ingested = Some(entry.created_at),
            None => last_ingested = Some(entry.created_at),
            _ => {}
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let avg_quality = if total_entries > 0 {
        quality_sum / total_entries as f32
    } else {
        0.0
    };

    KnowledgeStats {
        total_entries,
        trajectory_count,
        explicit_count,
        avg_quality,
        last_ingested,
    }
}
