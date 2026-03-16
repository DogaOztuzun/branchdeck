use crate::error::AppError;
use crate::models::{FileStatus, RepoInfo, WorktreeInfo, WorktreePreview};
use git2::{Repository, StatusOptions};
use log::{debug, error, info, warn};
use std::path::Path;

pub fn sanitize_worktree_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse consecutive dashes
    let mut result = String::with_capacity(sanitized.len());
    let mut prev_dash = false;
    for c in sanitized.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            result.push(c);
        }
    }

    // Trim leading/trailing dashes and underscores
    let result = result.trim_matches(|c: char| c == '-' || c == '_');

    // Truncate to 80 characters
    let result = if result.len() > 80 {
        result[..80].trim_end_matches(['-', '_'])
    } else {
        result
    };

    // Reject git-reserved names
    if result.eq_ignore_ascii_case("head") {
        return String::new();
    }

    result.to_string()
}

pub fn validate_repo(path: &Path) -> Result<RepoInfo, AppError> {
    let repo = Repository::discover(path)?;

    let workdir = repo
        .workdir()
        .ok_or_else(|| AppError::Git(git2::Error::from_str("Bare repository not supported")))?;

    let name = workdir
        .file_name()
        .or_else(|| workdir.parent().and_then(|p| p.file_name()))
        .map_or_else(
            || "unknown".to_string(),
            |n| n.to_string_lossy().to_string(),
        );

    let current_branch = match repo.head() {
        Ok(head) => head.shorthand().unwrap_or("HEAD").to_string(),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => "main".to_string(),
        Err(e) => return Err(e.into()),
    };

    debug!("Validated repo: {name} at {}", workdir.display());

    Ok(RepoInfo {
        name,
        path: workdir.to_path_buf(),
        current_branch,
    })
}

pub fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>, AppError> {
    let repo = Repository::open(repo_path)?;
    let mut result = Vec::new();

    // Add main worktree
    if let Some(workdir) = repo.workdir() {
        let branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "HEAD".to_string());

        let name = workdir
            .file_name()
            .map_or_else(|| "main".to_string(), |n| n.to_string_lossy().to_string());

        result.push(WorktreeInfo {
            name,
            path: workdir.to_path_buf(),
            branch,
            is_main: true,
        });
    }

    // Add linked worktrees
    let worktrees = repo.worktrees().map_err(|e| {
        error!("Failed to list worktrees for {}: {e}", repo_path.display());
        e
    })?;
    for wt_name in worktrees.iter().flatten() {
        let wt = repo.find_worktree(wt_name)?;
        let wt_repo = Repository::open_from_worktree(&wt)?;

        let branch = wt_repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "HEAD".to_string());

        let path = wt_repo.workdir().map(Path::to_path_buf).unwrap_or_default();

        result.push(WorktreeInfo {
            name: wt_name.to_string(),
            path,
            branch,
            is_main: false,
        });
    }

    debug!("Listed {} worktrees for {}", result.len(), repo_path.display());

    Ok(result)
}

pub fn create_worktree(
    repo_path: &Path,
    name: &str,
    branch: Option<&str>,
) -> Result<WorktreeInfo, AppError> {
    let sanitized = sanitize_worktree_name(name);
    if sanitized.is_empty() {
        error!("Worktree name is empty after sanitization: {name:?}");
        return Err(AppError::Git(git2::Error::from_str(
            "Worktree name must contain at least one letter or number",
        )));
    }

    let repo = Repository::open(repo_path)?;

    let worktree_dir = repo_path.parent().unwrap_or(repo_path).join("worktrees");
    std::fs::create_dir_all(&worktree_dir)?;
    let target_path = worktree_dir.join(&sanitized);

    let branch_name = branch.map_or_else(|| sanitized.clone(), String::from);

    // Create the branch from HEAD if it doesn't exist
    let head_commit = repo.head()?.peel_to_commit()?;
    let branch_ref = if repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .is_ok()
    {
        warn!("Branch {branch_name:?} already exists, reusing for worktree");
        format!("refs/heads/{branch_name}")
    } else {
        let new_branch = repo.branch(&branch_name, &head_commit, false)?;
        let ref_name = new_branch
            .into_reference()
            .name()
            .ok_or_else(|| AppError::Git(git2::Error::from_str("Invalid branch reference")))?
            .to_string();
        ref_name
    };

    let reference = repo.find_reference(&branch_ref)?;
    repo.worktree(
        &sanitized,
        &target_path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )?;

    info!("Created worktree {sanitized:?} on branch {branch_name:?} at {}", target_path.display());

    Ok(WorktreeInfo {
        name: sanitized,
        path: target_path,
        branch: branch_name,
        is_main: false,
    })
}

pub fn remove_worktree(
    repo_path: &Path,
    worktree_name: &str,
    delete_branch: bool,
) -> Result<(), AppError> {
    let repo = Repository::open(repo_path)?;

    let wt = repo.find_worktree(worktree_name)?;
    let wt_repo = Repository::open_from_worktree(&wt)?;
    let wt_path = wt_repo.workdir().map(Path::to_path_buf);
    let wt_branch = if delete_branch {
        wt_repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
    } else {
        None
    };
    drop(wt_repo);

    // Prune the worktree from git
    wt.prune(Some(
        git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))?;

    // Remove the worktree directory from disk
    if let Some(path) = wt_path {
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
    }

    // Delete the associated branch if requested
    if let Some(branch_name) = wt_branch {
        if let Ok(mut branch) = repo.find_branch(&branch_name, git2::BranchType::Local) {
            branch.delete()?;
        }
    }

    info!("Removed worktree {worktree_name:?} from {}", repo_path.display());

    Ok(())
}

pub fn preview_worktree(repo_path: &Path, name: &str) -> Result<WorktreePreview, AppError> {
    let sanitized = sanitize_worktree_name(name);
    let branch_name = sanitized.clone();
    let worktree_path = repo_path
        .parent()
        .unwrap_or(repo_path)
        .join("worktrees")
        .join(&sanitized);

    let repo = Repository::open(repo_path)?;

    let base_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "HEAD".to_string());

    let (branch_exists, worktree_exists) = if sanitized.is_empty() {
        (false, false)
    } else {
        let b_exists = repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .is_ok();
        let wt_exists = repo
            .worktrees()?
            .iter()
            .any(|name| name == Some(sanitized.as_str()));
        (b_exists, wt_exists)
    };
    let path_exists = !sanitized.is_empty() && worktree_path.exists();

    debug!(
        "Preview worktree {sanitized:?}: branch_exists={branch_exists}, path_exists={path_exists}, worktree_exists={worktree_exists}"
    );

    Ok(WorktreePreview {
        sanitized_name: sanitized,
        branch_name,
        worktree_path,
        base_branch,
        branch_exists,
        path_exists,
        worktree_exists,
    })
}

pub fn get_status(worktree_path: &Path) -> Result<Vec<FileStatus>, AppError> {
    let repo = Repository::open(worktree_path)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    let result: Vec<FileStatus> = statuses
        .iter()
        .map(|entry| {
            let path = entry.path().unwrap_or("").to_string();
            let status = format_status(entry.status());
            FileStatus { path, status }
        })
        .collect();

    debug!("Got {} status entries for {}", result.len(), worktree_path.display());

    Ok(result)
}

fn format_status(status: git2::Status) -> String {
    if status.contains(git2::Status::WT_NEW) || status.contains(git2::Status::INDEX_NEW) {
        "new".to_string()
    } else if status.contains(git2::Status::WT_MODIFIED)
        || status.contains(git2::Status::INDEX_MODIFIED)
    {
        "modified".to_string()
    } else if status.contains(git2::Status::WT_DELETED)
        || status.contains(git2::Status::INDEX_DELETED)
    {
        "deleted".to_string()
    } else if status.contains(git2::Status::WT_RENAMED)
        || status.contains(git2::Status::INDEX_RENAMED)
    {
        "renamed".to_string()
    } else if status.contains(git2::Status::CONFLICTED) {
        "conflicted".to_string()
    } else {
        "unknown".to_string()
    }
}
