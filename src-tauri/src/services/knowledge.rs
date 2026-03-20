//! Core `KnowledgeService` — manages RVF stores and provides the knowledge API.
//!
//! This service is always used as `Arc<KnowledgeService>` in Tauri state.
//! Methods that spawn async tasks take `self: &Arc<Self>`.

#[cfg(feature = "knowledge")]
use rvf_runtime::{MetadataEntry, MetadataValue, RvfOptions, RvfStore};

use crate::error::AppError;
use crate::models::knowledge::{KnowledgeEntry, KnowledgeType, PendingEntry, TrajectoryRecord};
use log::{debug, error, info, warn};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// ONNX embedding dimension for BAAI/bge-small-en-v1.5
pub(crate) const EMBEDDING_DIM: u16 = 384;

/// Maximum number of entries in the embed queue before dropping oldest.
pub(crate) const MAX_EMBED_QUEUE_SIZE: usize = 1000;

/// Metadata field IDs used in RVF stores.
pub(crate) mod field_ids {
    /// `worktree_id` (String) — for cascading query filter
    pub const WORKTREE_ID: u16 = 0;
    /// `entry_type` (String) — "trajectory", "commit", "explicit", etc.
    pub const ENTRY_TYPE: u16 = 1;
    /// `quality_score` (F64)
    pub const QUALITY_SCORE: u16 = 2;
    /// `created_at` (U64) — epoch milliseconds
    pub const CREATED_AT: u16 = 3;
}

/// Wrapper around `RvfStore` + content index.
/// RVF `SearchResult` only returns `{id, distance}`, so we maintain
/// a parallel content index for full entry retrieval.
#[cfg(feature = "knowledge")]
pub struct StoreHandle {
    pub(crate) store: RvfStore,
    /// Content index: `vector_id` → `KnowledgeEntry`
    pub(crate) entries: HashMap<u64, KnowledgeEntry>,
    /// Path to the JSONL content index file
    pub(crate) entries_path: PathBuf,
    /// Next vector ID to assign
    next_id: u64,
}

#[cfg(feature = "knowledge")]
impl StoreHandle {
    pub(crate) fn create_or_open(rvf_path: &Path, entries_path: &Path) -> Result<Self, AppError> {
        let store = if rvf_path.exists() {
            RvfStore::open(rvf_path).map_err(|e| {
                error!("Failed to open RVF store at {}: {e}", rvf_path.display());
                AppError::Knowledge(format!("Failed to open RVF: {e}"))
            })?
        } else {
            let options = RvfOptions {
                dimension: EMBEDDING_DIM,
                ..RvfOptions::default()
            };
            RvfStore::create(rvf_path, options).map_err(|e| {
                error!("Failed to create RVF store at {}: {e}", rvf_path.display());
                AppError::Knowledge(format!("Failed to create RVF: {e}"))
            })?
        };

        let entries = load_content_index(entries_path);
        let next_id = entries.keys().max().map_or(1, |max| max + 1);

        debug!(
            "Opened store at {} with {} entries",
            rvf_path.display(),
            entries.len()
        );

        Ok(Self {
            store,
            entries,
            entries_path: entries_path.to_path_buf(),
            next_id,
        })
    }

    pub(crate) fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn ingest(
        &mut self,
        embedding: &[f32],
        entry: KnowledgeEntry,
    ) -> Result<u64, AppError> {
        let id = entry.id;
        let metadata = build_metadata(
            entry.worktree_id.as_deref(),
            &entry.entry_type,
            entry.metadata.quality_score,
            entry.created_at,
        );

        self.store
            .ingest_batch(&[embedding], &[id], Some(&metadata))
            .map_err(|e| {
                error!("RVF ingest failed for id {id}: {e}");
                AppError::Knowledge(format!("Ingest failed: {e}"))
            })?;

        // Persist to content index
        append_content_entry(&self.entries_path, &entry);
        self.entries.insert(id, entry);

        Ok(id)
    }
}

pub struct KnowledgeService {
    #[cfg(feature = "knowledge")]
    global_store: Arc<RwLock<StoreHandle>>,
    #[cfg(feature = "knowledge")]
    pub(crate) repo_stores: Arc<RwLock<HashMap<String, Arc<RwLock<StoreHandle>>>>>,
    pub(crate) active_trajectories: Arc<RwLock<HashMap<String, TrajectoryRecord>>>,
    pub(crate) embed_queue: Arc<RwLock<Vec<PendingEntry>>>,
    #[cfg(feature = "knowledge")]
    embedder: Arc<RwLock<Option<fastembed::TextEmbedding>>>,
    #[cfg(feature = "knowledge")]
    shutdown_token: tokio_util::sync::CancellationToken,
    #[cfg(feature = "sona")]
    sona_engine: ruvector_sona::SonaEngine,
    #[cfg(feature = "sona")]
    pub(crate) persisted_pattern_ids: RwLock<std::collections::HashSet<u64>>,
    config_dir: PathBuf,
}

impl KnowledgeService {
    /// Create a new `KnowledgeService`. Eagerly opens/creates `global.rvf`.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if the global store cannot be created.
    #[cfg(feature = "knowledge")]
    pub fn new(config_dir: &Path) -> Result<Self, AppError> {
        let global_rvf_path = config_dir.join("global.rvf");
        let global_entries_path = config_dir.join("global.entries.jsonl");
        let global_store = StoreHandle::create_or_open(&global_rvf_path, &global_entries_path)?;

        info!(
            "KnowledgeService initialized, global store at {}",
            global_rvf_path.display()
        );

        #[cfg(feature = "sona")]
        let sona_engine = {
            let config = ruvector_sona::SonaConfig {
                hidden_dim: usize::from(EMBEDDING_DIM),
                embedding_dim: usize::from(EMBEDDING_DIM),
                ..ruvector_sona::SonaConfig::default()
            };
            ruvector_sona::SonaEngine::with_config(config)
        };

        let service = Self {
            global_store: Arc::new(RwLock::new(global_store)),
            repo_stores: Arc::new(RwLock::new(HashMap::new())),
            active_trajectories: Arc::new(RwLock::new(HashMap::new())),
            embed_queue: Arc::new(RwLock::new(Vec::new())),
            embedder: Arc::new(RwLock::new(None)),
            shutdown_token: tokio_util::sync::CancellationToken::new(),
            #[cfg(feature = "sona")]
            sona_engine,
            #[cfg(feature = "sona")]
            persisted_pattern_ids: RwLock::new(std::collections::HashSet::new()),
            config_dir: config_dir.to_path_buf(),
        };

        // Load persisted embed queue synchronously (RwLock not yet shared)
        let queue_path = config_dir.join("embed_queue.jsonl");
        if queue_path.exists() {
            let loaded = load_embed_queue(&queue_path);
            if !loaded.is_empty() {
                info!("Loaded {} pending embed queue entries", loaded.len());
                // Safe to use blocking_write since service isn't shared yet
                service.embed_queue.blocking_write().extend(loaded);
            }
        }

        Ok(service)
    }

    /// Open or create a repo-specific RVF store.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if the store cannot be opened/created.
    #[cfg(feature = "knowledge")]
    pub async fn open_repo(&self, repo_path: &str) -> Result<(), AppError> {
        let hash = repo_hash(repo_path);
        let rvf_path = self.config_dir.join(format!("{hash}.rvf"));
        let entries_path = self.config_dir.join(format!("{hash}.entries.jsonl"));

        let handle = StoreHandle::create_or_open(&rvf_path, &entries_path)?;
        info!("Opened repo knowledge store for {repo_path:?}");

        let mut stores = self.repo_stores.write().await;
        stores.insert(hash, Arc::new(RwLock::new(handle)));
        Ok(())
    }

    /// Close a repo-specific RVF store, flushing to disk.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if the store is still in use or cannot be closed.
    #[cfg(feature = "knowledge")]
    pub async fn close_repo(&self, repo_path: &str) -> Result<(), AppError> {
        let hash = repo_hash(repo_path);
        let mut stores = self.repo_stores.write().await;

        // Check refcount before removing from map
        if let Some(handle) = stores.get(&hash) {
            if Arc::strong_count(handle) > 1 {
                return Err(AppError::Knowledge(
                    "Store still in use, cannot close".to_string(),
                ));
            }
        }

        if let Some(handle) = stores.remove(&hash) {
            match Arc::try_unwrap(handle) {
                Ok(store_handle) => {
                    let sh = store_handle.into_inner();
                    sh.store.close().map_err(|e| {
                        error!("Failed to close repo store for {repo_path:?}: {e}");
                        AppError::Knowledge(format!("Failed to close store: {e}"))
                    })?;
                    info!("Closed repo knowledge store for {repo_path:?}");
                }
                Err(arc) => {
                    // Put it back in the map — we couldn't unwrap
                    stores.insert(hash, arc);
                    return Err(AppError::Knowledge(
                        "Store still in use, cannot close".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Close all repo stores and the global store. Called on app shutdown.
    #[cfg(feature = "knowledge")]
    pub async fn close_all(&self) {
        // Signal subscriber and tick loop to stop
        self.shutdown_token.cancel();
        // Brief yield to let spawned tasks observe cancellation and drop their Arc refs
        tokio::task::yield_now().await;

        // Force SONA to extract remaining patterns before closing stores.
        // Note: force_learn() blocks the current thread (K-means++ clustering).
        // Accepted at shutdown — SonaEngine is not Clone so spawn_blocking
        // would require Arc<SonaEngine> restructuring. App is exiting anyway.
        #[cfg(feature = "sona")]
        {
            let msg = self.sona_engine.force_learn();
            info!("[sona] Shutdown extraction: {msg}");
            self.persist_sona_patterns().await;
        }

        // Persist embed queue
        let queue_path = self.config_dir.join("embed_queue.jsonl");
        let queue = self.embed_queue.read().await;
        if !queue.is_empty() {
            persist_embed_queue(&queue_path, &queue);
            info!("Persisted {} embed queue entries on shutdown", queue.len());
        }

        // Close repo stores
        let mut stores = self.repo_stores.write().await;
        let keys: Vec<String> = stores.keys().cloned().collect();
        for key in keys {
            if let Some(handle) = stores.remove(&key) {
                if let Ok(store_handle) = Arc::try_unwrap(handle) {
                    let sh = store_handle.into_inner();
                    if let Err(e) = sh.store.close() {
                        error!("Failed to close repo store {key}: {e}");
                    }
                }
            }
        }

        // Global store close: RvfStore::close() takes ownership (self), but
        // self.global_store is a struct field we can't move out of. Data is safe —
        // RVF fsyncs on each ingest. OS reclaims resources on process exit.

        debug!("All knowledge stores closed");
    }

    /// Lazy-load the ONNX embedding model.
    /// Returns `true` if embedder is available, `false` if it couldn't be loaded.
    #[cfg(feature = "knowledge")]
    pub async fn ensure_embedder(&self) -> bool {
        // Fast path: read lock only
        {
            let guard = self.embedder.read().await;
            if guard.is_some() {
                return true;
            }
        }

        // Slow path: write lock to initialize
        let mut guard = self.embedder.write().await;
        // Double-check after acquiring write lock
        if guard.is_some() {
            return true;
        }

        let cache_dir = self.config_dir.join("models");
        let options = fastembed::TextInitOptions::default().with_cache_dir(cache_dir);

        info!("Initializing ONNX embedding model (first use)...");
        match fastembed::TextEmbedding::try_new(options) {
            Ok(model) => {
                info!("ONNX embedding model loaded successfully");
                *guard = Some(model);
                drop(guard);
                // Drain any queued entries now that embedder is available
                let queue_len = self.embed_queue.read().await.len();
                if queue_len > 0 {
                    info!("Embedder available, draining {queue_len} queued entries");
                    if let Err(e) = self.drain_embed_queue().await {
                        warn!("Failed to drain embed queue: {e}");
                    }
                }
                true
            }
            Err(e) => {
                warn!("Failed to load ONNX embedding model: {e}");
                false
            }
        }
    }

    /// Embed text using the ONNX model. Returns None if model unavailable.
    #[cfg(feature = "knowledge")]
    pub async fn embed_text(&self, text: &str) -> Option<Vec<f32>> {
        let mut guard = self.embedder.write().await;
        let model = guard.as_mut()?;
        match model.embed(vec![text], None) {
            Ok(mut embeddings) => embeddings.pop(),
            Err(e) => {
                error!("Embedding failed: {e}");
                None
            }
        }
    }

    /// Drain the embed queue — embed and store all queued entries.
    /// Returns the number of entries processed.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Knowledge` if embedding or storage fails.
    #[cfg(feature = "knowledge")]
    pub async fn drain_embed_queue(&self) -> Result<usize, AppError> {
        let entries: Vec<PendingEntry> = {
            let mut queue = self.embed_queue.write().await;
            std::mem::take(&mut *queue)
        };

        if entries.is_empty() {
            return Ok(0);
        }

        let mut processed = 0;
        let mut failed_entries: Vec<PendingEntry> = Vec::new();

        for pending in entries {
            if let Some(embedding) = self.embed_text(&pending.content).await {
                let repo_stores = self.repo_stores.read().await;
                let store_lock = if pending.repo_hash.is_empty() {
                    Some(Arc::clone(&self.global_store))
                } else {
                    repo_stores.get(&pending.repo_hash).map(Arc::clone)
                };

                if let Some(lock) = store_lock {
                    let mut store = lock.write().await;
                    let id = store.allocate_id();
                    let entry = KnowledgeEntry {
                        id,
                        content: pending.content.clone(),
                        entry_type: pending.entry_type.clone(),
                        source: pending.source.clone(),
                        repo_hash: pending.repo_hash.clone(),
                        worktree_id: pending.worktree_id.clone(),
                        metadata: pending.metadata.clone(),
                        created_at: pending.created_at,
                    };
                    if let Err(e) = store.ingest(embedding.as_slice(), entry) {
                        error!("Failed to drain queued entry: {e}");
                        failed_entries.push(pending);
                    } else {
                        processed += 1;
                    }
                } else {
                    failed_entries.push(pending);
                }
            } else {
                failed_entries.push(pending);
            }
        }

        // Re-queue entries that actually failed
        if !failed_entries.is_empty() {
            warn!(
                "{} embed queue entries could not be processed, re-queuing",
                failed_entries.len()
            );
            let mut queue = self.embed_queue.write().await;
            queue.extend(failed_entries);
        }

        // Clear persisted queue file
        let queue_path = self.config_dir.join("embed_queue.jsonl");
        if queue_path.exists() {
            let _ = std::fs::remove_file(&queue_path);
        }

        info!("Drained {processed} entries from embed queue");
        Ok(processed)
    }

    /// Get a reference to the repo store for a given repo hash.
    #[cfg(feature = "knowledge")]
    pub(crate) async fn get_repo_store(&self, repo_hash: &str) -> Option<Arc<RwLock<StoreHandle>>> {
        let stores = self.repo_stores.read().await;
        stores.get(repo_hash).map(Arc::clone)
    }

    /// Get a reference to the global store.
    #[cfg(feature = "knowledge")]
    pub(crate) fn global_store(&self) -> &Arc<RwLock<StoreHandle>> {
        &self.global_store
    }

    /// Get the config directory path.
    #[allow(dead_code)]
    pub(crate) fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get the shutdown cancellation token.
    #[cfg(feature = "knowledge")]
    pub(crate) fn shutdown_token(&self) -> &tokio_util::sync::CancellationToken {
        &self.shutdown_token
    }

    /// Get a reference to the SONA engine.
    #[cfg(feature = "sona")]
    pub(crate) fn sona(&self) -> &ruvector_sona::SonaEngine {
        &self.sona_engine
    }
}

/// Compute SHA256 hash of a repo path (same pattern as `config::repo_config_path`).
#[must_use]
pub fn repo_hash(repo_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(repo_path.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build metadata entries for a single vector ingest.
#[cfg(feature = "knowledge")]
fn build_metadata(
    worktree_id: Option<&str>,
    entry_type: &KnowledgeType,
    quality_score: f32,
    created_at: u64,
) -> Vec<MetadataEntry> {
    let mut entries = Vec::with_capacity(4);

    if let Some(wt_id) = worktree_id {
        entries.push(MetadataEntry {
            field_id: field_ids::WORKTREE_ID,
            value: MetadataValue::String(wt_id.to_string()),
        });
    }

    let type_str = match entry_type {
        KnowledgeType::Trajectory => "trajectory",
        KnowledgeType::Commit => "commit",
        KnowledgeType::Explicit => "explicit",
        KnowledgeType::ErrorResolution => "error_resolution",
        KnowledgeType::Pattern => "pattern",
    };
    entries.push(MetadataEntry {
        field_id: field_ids::ENTRY_TYPE,
        value: MetadataValue::String(type_str.to_string()),
    });

    entries.push(MetadataEntry {
        field_id: field_ids::QUALITY_SCORE,
        value: MetadataValue::F64(f64::from(quality_score)),
    });

    entries.push(MetadataEntry {
        field_id: field_ids::CREATED_AT,
        value: MetadataValue::U64(created_at),
    });

    entries
}

/// Load content index from a JSONL file.
fn load_content_index(path: &Path) -> HashMap<u64, KnowledgeEntry> {
    let mut entries = HashMap::new();
    if !path.exists() {
        return entries;
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read content index {}: {e}", path.display());
            return entries;
        }
    };

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<KnowledgeEntry>(line) {
            Ok(entry) => {
                entries.insert(entry.id, entry);
            }
            Err(e) => {
                // Skip unparseable lines (crash resilience)
                debug!("Skipping unparseable content index line: {e}");
            }
        }
    }

    entries
}

/// Append a single entry to the content index JSONL file.
fn append_content_entry(path: &Path, entry: &KnowledgeEntry) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        error!(
            "Failed to open content index for append: {}",
            path.display()
        );
        return;
    };

    let Ok(json) = serde_json::to_string(entry) else {
        error!("Failed to serialize knowledge entry");
        return;
    };

    if let Err(e) = writeln!(file, "{json}") {
        error!("Failed to append to content index: {e}");
        return;
    }

    if let Err(e) = file.sync_all() {
        warn!("Failed to fsync content index: {e}");
    }
}

/// Load persisted embed queue from JSONL.
fn load_embed_queue(path: &Path) -> Vec<PendingEntry> {
    let mut entries = Vec::new();
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read embed queue {}: {e}", path.display());
            return entries;
        }
    };

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<PendingEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                debug!("Skipping unparseable embed queue line: {e}");
            }
        }
    }

    entries
}

/// Persist embed queue to JSONL file.
fn persist_embed_queue(path: &Path, queue: &[PendingEntry]) {
    use std::io::Write;

    let Ok(mut file) = std::fs::File::create(path) else {
        error!("Failed to create embed queue file: {}", path.display());
        return;
    };

    for entry in queue {
        let Ok(json) = serde_json::to_string(entry) else {
            continue;
        };
        if let Err(e) = writeln!(file, "{json}") {
            error!("Failed to write embed queue entry: {e}");
            return;
        }
    }

    if let Err(e) = file.sync_all() {
        warn!("Failed to fsync embed queue: {e}");
    }
}
