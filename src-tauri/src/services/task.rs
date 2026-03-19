use crate::error::AppError;
use crate::models::task::{TaskFrontmatter, TaskInfo, TaskScope, TaskStatus, TaskType};
use log::{debug, error, info};
use std::io::Write;
use std::path::Path;
use yaml_front_matter::YamlFrontMatter;

/// Update the status field in a task.md file's YAML frontmatter.
///
/// Uses simple string replacement within the frontmatter section.
/// Logs errors but does not propagate them — task status on disk is
/// best-effort and must not break the run state machine.
pub fn update_task_status(task_path: &str, new_status: TaskStatus) {
    let status_str = match new_status {
        TaskStatus::Created => "created",
        TaskStatus::Running => "running",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Succeeded => "succeeded",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    };

    let content = match std::fs::read_to_string(task_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read task file for status update {task_path}: {e}");
            return;
        }
    };

    let Some(updated) = replace_frontmatter_status(&content, status_str) else {
        error!("Failed to locate status field in frontmatter of {task_path}");
        return;
    };

    if let Err(e) = std::fs::write(task_path, updated) {
        error!("Failed to write updated task status to {task_path}: {e}");
    } else {
        debug!("Updated task status to {status_str} in {task_path}");
    }
}

/// Replace the `status: <value>` line in YAML frontmatter.
/// Returns `None` if the frontmatter or status field cannot be found.
#[must_use]
pub fn replace_frontmatter_status(content: &str, new_status: &str) -> Option<String> {
    // Frontmatter is delimited by `---\n` at start and `\n---\n` later
    let rest = content.strip_prefix("---\n")?;
    let end_idx = rest.find("\n---\n").or_else(|| rest.find("\n---"))?;
    let frontmatter = &rest[..end_idx];

    // Find and replace the status line
    let mut found = false;
    let new_fm: String = frontmatter
        .lines()
        .map(|line| {
            if line.starts_with("status:") {
                found = true;
                format!("status: {new_status}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !found {
        return None;
    }

    Some(format!("---\n{new_fm}{}", &rest[end_idx..]))
}

const TASK_DIR: &str = ".branchdeck";
const TASK_FILE: &str = "task.md";

/// Creates a new task in the given worktree path.
///
/// # Errors
///
/// Returns `TaskAlreadyExists` if `.branchdeck/task.md` already exists.
/// Returns `Io` if directory creation or file writing fails.
pub fn create_task(
    worktree_path: &str,
    task_type: TaskType,
    repo: &str,
    branch: &str,
    pr: Option<u64>,
) -> Result<TaskInfo, AppError> {
    let base = Path::new(worktree_path);
    let dir = base.join(TASK_DIR);
    let file_path = dir.join(TASK_FILE);

    std::fs::create_dir_all(&dir).map_err(|e| {
        error!("Failed to create .branchdeck dir at {}: {e}", dir.display());
        e
    })?;

    let created = chrono::Utc::now().to_rfc3339();
    let frontmatter = TaskFrontmatter {
        task_type,
        scope: TaskScope::Worktree,
        status: TaskStatus::Created,
        repo: repo.to_owned(),
        branch: branch.to_owned(),
        pr,
        created,
        run_count: 0,
    };

    let body = "\n## Goal\n\n\n\n## Progress\n\n- [ ] Identify approach\n- [ ] Implement\n- [ ] Verify\n\n## Log\n".to_owned();

    let content = format_task_md(&frontmatter, &body)?;

    // Atomic create: fails if the file already exists (TOCTOU-safe)
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                error!("Task already exists at {}", file_path.display());
                return AppError::TaskAlreadyExists(file_path.display().to_string());
            }
            error!("Failed to create task.md at {}: {e}", file_path.display());
            AppError::Io(e)
        })?;

    file.write_all(content.as_bytes()).map_err(|e| {
        error!("Failed to write task.md at {}: {e}", file_path.display());
        e
    })?;

    info!("Created task at {}", file_path.display());

    Ok(TaskInfo {
        frontmatter,
        body,
        path: file_path.display().to_string(),
    })
}

/// Reads and parses an existing task from a worktree.
///
/// # Errors
///
/// Returns `TaskNotFound` if the file does not exist.
/// Returns `TaskParseError` if the YAML frontmatter cannot be parsed.
pub fn get_task(worktree_path: &str) -> Result<TaskInfo, AppError> {
    let file_path = Path::new(worktree_path).join(TASK_DIR).join(TASK_FILE);

    if !file_path.exists() {
        return Err(AppError::TaskNotFound(file_path.display().to_string()));
    }

    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        error!("Failed to read task.md at {}: {e}", file_path.display());
        e
    })?;

    parse_task_md(&content, &file_path.display().to_string())
}

/// Lists tasks from multiple worktree paths, skipping those without a task file.
///
/// # Errors
///
/// Returns errors only for parse failures on existing task files.
pub fn list_tasks(worktree_paths: &[String]) -> Result<Vec<TaskInfo>, AppError> {
    let mut tasks = Vec::new();

    for wt_path in worktree_paths {
        let file_path = Path::new(wt_path).join(TASK_DIR).join(TASK_FILE);
        if !file_path.exists() {
            debug!("No task.md in worktree {wt_path}, skipping");
            continue;
        }

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                debug!(
                    "Failed to read task.md at {}, skipping: {e}",
                    file_path.display()
                );
                continue;
            }
        };

        match parse_task_md(&content, &file_path.display().to_string()) {
            Ok(task_info) => tasks.push(task_info),
            Err(e) => {
                debug!(
                    "Skipping unparseable task.md at {}: {e}",
                    file_path.display()
                );
            }
        }
    }

    debug!(
        "Listed {} tasks from {} worktrees",
        tasks.len(),
        worktree_paths.len()
    );
    Ok(tasks)
}

/// Parses a `task.md` file from its raw content.
///
/// # Errors
///
/// Returns `TaskParseError` if the YAML frontmatter is malformed.
pub fn parse_task_md(content: &str, path: &str) -> Result<TaskInfo, AppError> {
    let document: yaml_front_matter::Document<TaskFrontmatter> = YamlFrontMatter::parse(content)
        .map_err(|e| {
            error!("Failed to parse task frontmatter at {path}: {e}");
            AppError::TaskParseError(format!("{path}: {e}"))
        })?;

    debug!(
        "Parsed task at {path} with status {:?}",
        document.metadata.status
    );

    Ok(TaskInfo {
        frontmatter: document.metadata,
        body: document.content,
        path: path.to_owned(),
    })
}

/// Serializes a `TaskFrontmatter` and body into a task.md file content string.
///
/// # Errors
///
/// Returns `TaskParseError` if YAML serialization fails.
fn format_task_md(fm: &TaskFrontmatter, body: &str) -> Result<String, AppError> {
    let yaml = serde_yaml::to_string(fm).map_err(|e| {
        error!("Failed to serialize task frontmatter: {e}");
        AppError::TaskParseError(format!("serialization: {e}"))
    })?;
    let yaml = yaml.trim_end_matches('\n');
    Ok(format!("---\n{yaml}\n---\n{body}"))
}
