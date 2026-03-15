use crate::error::AppError;
use crate::models::{FileStatus, RepoInfo, WorktreeInfo};
use git2::{Repository, StatusOptions};
use std::path::Path;

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
    let worktrees = repo.worktrees()?;
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

    Ok(result)
}

pub fn create_worktree(
    repo_path: &Path,
    name: &str,
    branch: Option<&str>,
) -> Result<WorktreeInfo, AppError> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(AppError::Git(git2::Error::from_str(
            "Worktree name must not contain path separators or '..'",
        )));
    }

    let repo = Repository::open(repo_path)?;

    let target_path = repo_path.parent().unwrap_or(repo_path).join(name);

    let branch_name = branch.unwrap_or(name);

    // Create the branch from HEAD if it doesn't exist
    let head_commit = repo.head()?.peel_to_commit()?;
    let branch_ref = if repo
        .find_branch(branch_name, git2::BranchType::Local)
        .is_ok()
    {
        format!("refs/heads/{branch_name}")
    } else {
        let new_branch = repo.branch(branch_name, &head_commit, false)?;
        let ref_name = new_branch
            .into_reference()
            .name()
            .ok_or_else(|| AppError::Git(git2::Error::from_str("Invalid branch reference")))?
            .to_string();
        ref_name
    };

    let reference = repo.find_reference(&branch_ref)?;
    repo.worktree(
        name,
        &target_path,
        Some(git2::WorktreeAddOptions::new().reference(Some(&reference))),
    )?;

    Ok(WorktreeInfo {
        name: name.to_string(),
        path: target_path,
        branch: branch_name.to_string(),
        is_main: false,
    })
}

pub fn get_status(worktree_path: &Path) -> Result<Vec<FileStatus>, AppError> {
    let repo = Repository::open(worktree_path)?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;

    let result = statuses
        .iter()
        .map(|entry| {
            let path = entry.path().unwrap_or("").to_string();
            let status = format_status(entry.status());
            FileStatus { path, status }
        })
        .collect();

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
