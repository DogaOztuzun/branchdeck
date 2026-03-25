use crate::models::run::RunInfo;
use log::{debug, error, warn};
use std::path::Path;

const BRANCHDECK_DIR: &str = ".branchdeck";
const RUN_STATE_FILE: &str = "run.json";

/// Derive the `.branchdeck/` directory from a task path.
///
/// Task paths look like `/foo/bar/.branchdeck/task.md` — we strip the
/// filename to get the `.branchdeck/` directory.
fn branchdeck_dir_from_task_path(task_path: &str) -> &Path {
    Path::new(task_path)
        .parent()
        .unwrap_or_else(|| Path::new("."))
}

/// Persist a `RunInfo` to `.branchdeck/run.json` alongside the task file.
///
/// Best-effort: logs errors but never panics.
pub fn save_run_state(task_path: &str, run_info: &RunInfo) {
    let dir = branchdeck_dir_from_task_path(task_path);
    let file_path = dir.join(RUN_STATE_FILE);

    let json = match serde_json::to_string_pretty(run_info) {
        Ok(j) => j,
        Err(e) => {
            error!("Failed to serialize run state for {task_path}: {e}");
            return;
        }
    };

    if let Err(e) = crate::util::write_atomic(&file_path, json.as_bytes()) {
        error!("Failed to write run state to {}: {e}", file_path.display());
    } else {
        debug!("Saved run state to {}", file_path.display());
    }
}

/// Load a `RunInfo` from `.branchdeck/run.json` in the given worktree.
///
/// Returns `None` if the file does not exist or cannot be parsed.
/// Best-effort: logs errors but never panics.
#[must_use]
pub fn load_run_state(worktree_path: &str) -> Option<RunInfo> {
    let file_path = Path::new(worktree_path)
        .join(BRANCHDECK_DIR)
        .join(RUN_STATE_FILE);

    if !file_path.exists() {
        return None;
    }

    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read run state from {}: {e}", file_path.display());
            return None;
        }
    };

    match serde_json::from_str::<RunInfo>(&content) {
        Ok(info) => {
            debug!(
                "Loaded run state from {} with status {:?}",
                file_path.display(),
                info.status
            );
            Some(info)
        }
        Err(e) => {
            warn!(
                "Corrupt run state at {}, deleting: {e}",
                file_path.display()
            );
            // Best-effort cleanup of corrupt file
            if let Err(del_err) = std::fs::remove_file(&file_path) {
                error!(
                    "Failed to delete corrupt run state {}: {del_err}",
                    file_path.display()
                );
            }
            None
        }
    }
}

/// Remove the `.branchdeck/run.json` file for a task.
///
/// Best-effort: logs errors but never panics.
pub fn delete_run_state(task_path: &str) {
    let dir = branchdeck_dir_from_task_path(task_path);
    let file_path = dir.join(RUN_STATE_FILE);

    if !file_path.exists() {
        return;
    }

    if let Err(e) = std::fs::remove_file(&file_path) {
        error!("Failed to delete run state at {}: {e}", file_path.display());
    } else {
        debug!("Deleted run state at {}", file_path.display());
    }
}

/// Scan multiple worktree paths for `run.json` files and collect all found `RunInfo`s.
///
/// Best-effort: skips worktrees that have no run state or corrupt files.
#[must_use]
pub fn scan_all_run_states(worktree_paths: &[String]) -> Vec<RunInfo> {
    let mut results = Vec::new();

    for wt_path in worktree_paths {
        if let Some(run_info) = load_run_state(wt_path) {
            results.push(run_info);
        }
    }

    debug!(
        "Scanned {} worktrees, found {} run states",
        worktree_paths.len(),
        results.len()
    );
    results
}
