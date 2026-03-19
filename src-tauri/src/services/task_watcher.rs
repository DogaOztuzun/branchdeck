use crate::error::AppError;
use crate::services::task;
use log::{debug, error, info, trace, warn};
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::Mutex;

const TASK_DIR: &str = ".branchdeck";
const TASK_FILE: &str = "task.md";
const DEBOUNCE_MS: u64 = 500;

pub struct TaskWatcher {
    watcher: Option<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>>,
    watched_paths: Vec<PathBuf>,
}

impl TaskWatcher {
    #[must_use]
    pub fn new() -> Self {
        Self {
            watcher: None,
            watched_paths: Vec::new(),
        }
    }

    /// Start watching `.branchdeck/` directories for task file changes.
    ///
    /// # Errors
    ///
    /// Returns `TaskWatchError` if the file watcher cannot be created.
    pub fn start<R: tauri::Runtime>(
        &mut self,
        app_handle: &tauri::AppHandle<R>,
        worktree_paths: &[String],
    ) -> Result<(), AppError> {
        // Stop any existing watcher first
        self.stop();

        let handle = app_handle.clone();

        let mut debouncer = new_debouncer(
            Duration::from_millis(DEBOUNCE_MS),
            move |result: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            if event.kind == DebouncedEventKind::Any {
                                let path = event.path.clone();
                                let h = handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    handle_file_change(&h, &path);
                                });
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Task watcher error: {e}");
                    }
                }
            },
        )
        .map_err(|e| {
            error!("Failed to create task watcher: {e}");
            AppError::TaskWatchError(e.to_string())
        })?;

        let mut paths = Vec::new();
        for wt_path in worktree_paths {
            let dir = Path::new(wt_path).join(TASK_DIR);
            if dir.is_dir() {
                if let Err(e) = debouncer
                    .watcher()
                    .watch(&dir, notify::RecursiveMode::NonRecursive)
                {
                    warn!("Failed to watch {}: {e}", dir.display());
                } else {
                    debug!("Watching {}", dir.display());
                    paths.push(dir);
                }
            } else {
                debug!("Skipping non-existent .branchdeck dir: {}", dir.display());
            }
        }

        info!("TaskWatcher started, watching {} paths", paths.len());
        self.watched_paths = paths;
        self.watcher = Some(debouncer);
        Ok(())
    }

    /// Add a single `.branchdeck/` directory to the active watcher.
    ///
    /// Returns `true` if the path was newly added, `false` if already watched
    /// or if the directory does not exist.
    ///
    /// # Errors
    ///
    /// Returns `TaskWatchError` if no watcher is active or watching fails.
    pub fn watch_path(&mut self, worktree_path: &str) -> Result<bool, AppError> {
        let dir = Path::new(worktree_path).join(TASK_DIR);

        if !dir.is_dir() {
            debug!(
                "watch_path: .branchdeck dir does not exist at {}",
                dir.display()
            );
            return Ok(false);
        }

        if self.watched_paths.contains(&dir) {
            debug!("watch_path: already watching {}", dir.display());
            return Ok(false);
        }

        let debouncer = self.watcher.as_mut().ok_or_else(|| {
            error!("watch_path called with no active watcher");
            AppError::TaskWatchError("no active watcher".to_owned())
        })?;

        debouncer
            .watcher()
            .watch(&dir, notify::RecursiveMode::NonRecursive)
            .map_err(|e| {
                error!("Failed to watch {}: {e}", dir.display());
                AppError::TaskWatchError(e.to_string())
            })?;

        info!("Added watch for {}", dir.display());
        self.watched_paths.push(dir);
        Ok(true)
    }

    /// Stop the current watcher, dropping all watches.
    pub fn stop(&mut self) {
        if self.watcher.is_some() {
            info!(
                "TaskWatcher stopped, was watching {} paths",
                self.watched_paths.len()
            );
            self.watcher = None;
            self.watched_paths.clear();
        }
    }
}

impl Default for TaskWatcher {
    fn default() -> Self {
        Self::new()
    }
}

fn handle_file_change<R: tauri::Runtime>(handle: &tauri::AppHandle<R>, path: &Path) {
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if file_name != TASK_FILE {
        return;
    }

    trace!("Task file changed: {}", path.display());

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read changed task file {}: {e}", path.display());
            return;
        }
    };

    let path_str = path.display().to_string();
    match task::parse_task_md(&content, &path_str) {
        Ok(task_info) => {
            trace!("Emitting task:updated for {}", path.display());
            if let Err(e) = handle.emit("task:updated", &task_info) {
                error!("Failed to emit task:updated event: {e}");
            }
        }
        Err(e) => {
            warn!("Failed to parse changed task file {}: {e}", path.display());
        }
    }
}

/// Type alias for the managed state.
pub type TaskWatcherState = Arc<Mutex<TaskWatcher>>;

/// Create the initial `TaskWatcher` managed state.
#[must_use]
pub fn create_watcher_state() -> TaskWatcherState {
    Arc::new(Mutex::new(TaskWatcher::new()))
}
