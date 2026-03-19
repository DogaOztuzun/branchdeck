//! Knowledge ingestion — `EventBus` subscriber and trajectory recording.

use crate::models::agent::Event;
use crate::models::knowledge::{
    KnowledgeEntry, KnowledgeMetadata, KnowledgeSource, KnowledgeType, PendingEntry,
    TrajectoryRecord, TrajectoryStep,
};
use crate::services::event_bus::EventBus;
use crate::services::knowledge::{repo_hash, KnowledgeService};
use log::{debug, error, info, trace, warn};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type TrajectoryMap = HashMap<String, TrajectoryRecord>;

impl KnowledgeService {
    /// Subscribe to the `EventBus` for trajectory recording.
    pub fn start_subscriber(self: &Arc<Self>, event_bus: &EventBus) {
        let service = Arc::clone(self);
        let mut rx = event_bus.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => service.handle_event(event).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("KnowledgeService lagged, missed {n} events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("EventBus closed, stopping KnowledgeService subscriber");
                        break;
                    }
                }
            }
        });
        info!("KnowledgeService: EventBus subscriber started");
    }

    async fn handle_event(&self, event: Event) {
        match &event {
            Event::SessionStart {
                session_id,
                tab_id,
                ts,
                ..
            } => {
                self.begin_trajectory(session_id, tab_id, *ts).await;
            }
            Event::ToolStart {
                session_id,
                tool_name,
                file_path,
                ts,
                ..
            } => {
                self.add_trajectory_step(session_id, tool_name, file_path.as_deref(), false, *ts)
                    .await;
            }
            Event::ToolEnd {
                session_id,
                tool_name,
                file_path,
                ts,
                ..
            } => {
                let modified = matches!(tool_name.as_str(), "Write" | "Edit");
                self.add_trajectory_step(
                    session_id,
                    tool_name,
                    file_path.as_deref(),
                    modified,
                    *ts,
                )
                .await;
            }
            #[cfg(feature = "knowledge")]
            Event::RunComplete {
                session_id,
                status,
                cost_usd,
                elapsed_secs,
                ..
            } => {
                self.finalize_trajectory(session_id, status, *cost_usd, *elapsed_secs)
                    .await;
            }
            Event::SessionStop { session_id, .. } => {
                self.end_trajectory_fallback(session_id).await;
            }
            _ => {} // SubagentStart/Stop, Notification — v1 captures root-agent tools only
        }
    }

    async fn begin_trajectory(&self, session_id: &str, tab_id: &str, ts: u64) {
        let mut trajectories: tokio::sync::RwLockWriteGuard<'_, TrajectoryMap> =
            self.active_trajectories.write().await;
        if trajectories.contains_key(session_id) {
            warn!("Trajectory already exists for session {session_id}, replacing");
        }
        trajectories.insert(
            session_id.to_string(),
            TrajectoryRecord {
                session_id: session_id.to_string(),
                tab_id: tab_id.to_string(),
                steps: Vec::new(),
                quality_score: 0.0,
                started_at: ts,
                ended_at: None,
            },
        );
        debug!("Trajectory started: {session_id}");
    }

    async fn add_trajectory_step(
        &self,
        session_id: &str,
        tool_name: &str,
        file_path: Option<&str>,
        was_modified: bool,
        ts: u64,
    ) {
        let mut trajectories: tokio::sync::RwLockWriteGuard<'_, TrajectoryMap> =
            self.active_trajectories.write().await;
        if let Some(trajectory) = trajectories.get_mut(session_id) {
            trajectory.steps.push(TrajectoryStep {
                tool_name: tool_name.to_string(),
                file_path: file_path.map(str::to_string),
                was_modified,
                ts,
            });
            trace!("Trajectory step: {session_id} → {tool_name}");
        }
    }

    /// Finalize a trajectory when `RunComplete` is received from `RunManager`.
    #[cfg(feature = "knowledge")]
    #[allow(clippy::too_many_lines)]
    async fn finalize_trajectory(
        &self,
        session_id: &str,
        status: &str,
        cost_usd: f64,
        _elapsed_secs: u64,
    ) {
        let trajectory = {
            let mut trajectories: tokio::sync::RwLockWriteGuard<'_, TrajectoryMap> =
                self.active_trajectories.write().await;
            trajectories.remove(session_id)
        };

        let Some(mut trajectory) = trajectory else {
            debug!("No active trajectory for session {session_id}");
            return;
        };

        let now = crate::models::agent::now_ms();
        trajectory.ended_at = Some(now);

        // Quality scoring: 1.0 succeeded, 0.3 failed, 0.5 unknown
        trajectory.quality_score = match status {
            "succeeded" => 1.0,
            "failed" | "cancelled" => 0.3,
            _ => 0.5,
        };

        let unique_files: HashSet<&str> = trajectory
            .steps
            .iter()
            .filter_map(|s| s.file_path.as_deref())
            .collect();

        let tool_names: Vec<String> = {
            let mut names: HashSet<String> = HashSet::new();
            for step in &trajectory.steps {
                names.insert(step.tool_name.clone());
            }
            names.into_iter().collect()
        };

        let file_paths: Vec<String> = unique_files.iter().map(|s| (*s).to_string()).collect();

        // Build summary text for embedding
        let summary = format!(
            "Run {status}: {} tool calls, {} files touched. Tools: {}. Files: {}",
            trajectory.steps.len(),
            unique_files.len(),
            tool_names.join(", "),
            if file_paths.is_empty() {
                "none".to_string()
            } else {
                file_paths.join(", ")
            }
        );

        let metadata = KnowledgeMetadata {
            session_id: Some(session_id.to_string()),
            tool_names,
            file_paths: file_paths.clone(),
            run_status: Some(status.to_string()),
            cost_usd: Some(cost_usd),
            quality_score: trajectory.quality_score,
        };

        // Try to embed and store
        if self.ensure_embedder().await {
            if let Some(embedding) = self.embed_text(&summary).await {
                // Find the repo store for this trajectory's tab
                // For now, use global store as fallback since we don't
                // have repo context from the event alone
                let store_lock = Arc::clone(self.global_store());
                let mut store = store_lock.write().await;
                let id = store.allocate_id();

                let entry = KnowledgeEntry {
                    id,
                    content: summary.clone(),
                    entry_type: KnowledgeType::Trajectory,
                    source: KnowledgeSource::EventBus,
                    repo_hash: String::new(), // global
                    worktree_id: None,
                    metadata,
                    created_at: now,
                };

                match store.ingest(embedding.as_slice(), entry) {
                    Ok(_) => {
                        info!(
                            "[knowledge] Stored trajectory: {} tool calls, {} files touched, quality: {}",
                            trajectory.steps.len(),
                            unique_files.len(),
                            trajectory.quality_score
                        );
                        // Append audit log
                        append_audit_entry(
                            self.config_dir(),
                            "",
                            session_id,
                            trajectory.quality_score,
                            &file_paths,
                        );
                    }
                    Err(e) => {
                        error!("Failed to store trajectory: {e}");
                    }
                }
            }
        } else {
            // Queue for later embedding
            let pending = PendingEntry {
                content: summary,
                entry_type: KnowledgeType::Trajectory,
                source: KnowledgeSource::EventBus,
                repo_hash: String::new(),
                worktree_id: None,
                metadata,
                created_at: now,
            };
            let mut queue = self.embed_queue.write().await;
            if queue.len() < crate::services::knowledge::MAX_EMBED_QUEUE_SIZE {
                queue.push(pending);
                debug!("Queued trajectory for later embedding (ONNX unavailable)");
            } else {
                warn!("Embed queue full, dropping trajectory entry");
            }
        }
    }

    /// Fallback trajectory end for sessions without `RunComplete`
    /// (e.g., plain Claude Code terminal sessions, or when `RunComplete`
    /// was emitted with empty `session_id` due to pre-session crash).
    async fn end_trajectory_fallback(&self, session_id: &str) {
        let has_trajectory = {
            let trajectories: tokio::sync::RwLockReadGuard<'_, TrajectoryMap> =
                self.active_trajectories.read().await;
            trajectories.contains_key(session_id)
        };

        if has_trajectory {
            #[cfg(feature = "knowledge")]
            self.finalize_trajectory(session_id, "unknown", 0.0, 0)
                .await;
            #[cfg(not(feature = "knowledge"))]
            {
                let mut trajectories: tokio::sync::RwLockWriteGuard<'_, TrajectoryMap> =
                    self.active_trajectories.write().await;
                trajectories.remove(session_id);
            }
        }
    }

    /// Explicit knowledge ingestion — called by MCP `remember_this` and Tauri IPC.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if embedding fails or the queue is full.
    #[cfg(feature = "knowledge")]
    pub async fn ingest_explicit(
        &self,
        repo_path: &str,
        worktree_id: Option<&str>,
        content: &str,
        entry_type: KnowledgeType,
    ) -> Result<u64, crate::error::AppError> {
        let hash = repo_hash(repo_path);
        let now = crate::models::agent::now_ms();

        let metadata = KnowledgeMetadata {
            session_id: None,
            tool_names: Vec::new(),
            file_paths: Vec::new(),
            run_status: None,
            cost_usd: None,
            quality_score: 1.0, // Explicit = highest quality
        };

        if self.ensure_embedder().await {
            if let Some(embedding) = self.embed_text(content).await {
                let store_lock = self
                    .get_repo_store(&hash)
                    .await
                    .unwrap_or_else(|| Arc::clone(self.global_store()));
                let mut store = store_lock.write().await;
                let id = store.allocate_id();

                let entry = KnowledgeEntry {
                    id,
                    content: content.to_string(),
                    entry_type,
                    source: KnowledgeSource::Mcp,
                    repo_hash: hash,
                    worktree_id: worktree_id.map(str::to_string),
                    metadata,
                    created_at: now,
                };

                store.ingest(embedding.as_slice(), entry)?;
                info!("[knowledge] Explicit entry stored: id={id}");
                return Ok(id);
            }
        }

        // Queue for later
        let pending = PendingEntry {
            content: content.to_string(),
            entry_type,
            source: KnowledgeSource::Mcp,
            repo_hash: hash,
            worktree_id: worktree_id.map(str::to_string),
            metadata,
            created_at: now,
        };
        let mut queue = self.embed_queue.write().await;
        if queue.len() >= crate::services::knowledge::MAX_EMBED_QUEUE_SIZE {
            return Err(crate::error::AppError::Knowledge(
                "Embed queue full, cannot queue entry".to_string(),
            ));
        }
        queue.push(pending);
        warn!("Queued explicit entry for later embedding");
        Ok(0) // ID 0 indicates queued, not yet stored
    }
}

/// Append an audit log entry to the repo's `audit.jsonl` file.
/// Uses the global audit log if `repo_hash` is empty.
fn append_audit_entry(
    config_dir: &std::path::Path,
    repo_hash: &str,
    session_id: &str,
    quality_score: f32,
    files_touched: &[String],
) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let filename = if repo_hash.is_empty() {
        "global.audit.jsonl".to_string()
    } else {
        format!("{repo_hash}.audit.jsonl")
    };

    let path = config_dir.join(filename);

    let entry = serde_json::json!({
        "action": "trajectory_complete",
        "session_id": session_id,
        "quality_score": quality_score,
        "timestamp": crate::models::agent::now_ms(),
        "files_touched": files_touched,
    });

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        error!("Failed to open audit log: {}", path.display());
        return;
    };

    let Ok(json) = serde_json::to_string(&entry) else {
        return;
    };

    if let Err(e) = writeln!(file, "{json}") {
        error!("Failed to write audit entry: {e}");
        return;
    }

    if let Err(e) = file.sync_all() {
        warn!("Failed to fsync audit log: {e}");
    }
}
