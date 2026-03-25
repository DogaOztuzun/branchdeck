use crate::error::AppError;
use crate::models::task::{TaskInfo, TaskType};
use crate::services::{git, github, task};
use log::{debug, error, info};
use std::fmt::Write as _;
use std::path::Path;

/// Create a pr-shepherd worktree + task for a given PR.
///
/// This is the orchestration function that:
/// 1. Fetches PR metadata
/// 2. Checks if a worktree already exists for the PR branch
/// 3. Creates a worktree from the PR branch
/// 4. Generates a pr-shepherd task.md with full context
///
/// Does NOT launch the run — that's a separate step.
///
/// # Errors
/// Returns `AppError` on GitHub API failure, git errors, or if a worktree/task already exists.
pub async fn shepherd_pr(
    repo_path: &str,
    pr_number: u64,
    #[cfg(feature = "knowledge")] knowledge: Option<
        &std::sync::Arc<crate::services::knowledge::KnowledgeService>,
    >,
) -> Result<ShepherdResult, AppError> {
    let repo = Path::new(repo_path);

    // 1. Resolve owner/repo
    let (owner, repo_name) = github::resolve_owner_repo(repo)?;

    // 2. Find the PR by number
    let pr_info = find_pr_by_number(&owner, &repo_name, pr_number).await?;

    let branch = &pr_info.branch;

    // 3. Check if worktree already exists for this branch — reuse if so
    let existing_worktrees = git::list_worktrees(repo)?;
    let existing_wt = existing_worktrees.iter().find(|wt| wt.branch == *branch);

    let (worktree_path, created_worktree_name) = if let Some(wt) = existing_wt {
        let wt_path = wt.path.display().to_string();
        debug!("Reusing existing worktree for branch {branch} at {wt_path}");

        // If task already exists in this worktree, return it directly
        if let Ok(existing_task) = task::get_task(&wt_path) {
            info!("Shepherd PR #{pr_number}: reusing existing task in {wt_path}");
            return Ok(ShepherdResult {
                task: existing_task,
                worktree_path: wt_path,
                knowledge_recalled: 0,
            });
        }
        (wt_path, None)
    } else {
        // 4. Fetch the remote branch if not local
        fetch_branch_if_needed(repo, branch)?;

        // 5. Create worktree
        let worktree = git::create_worktree(repo, branch, Some(branch), None)?;
        let name = worktree.name.clone();
        (worktree.path.display().to_string(), Some(name))
    };

    // 7. Query knowledge for prior fix patterns
    let mut knowledge_context = String::new();
    let mut knowledge_recalled: u32 = 0;

    #[cfg(feature = "knowledge")]
    if let Some(ks) = knowledge {
        let query_text = format!(
            "CI fix pattern for {} failing checks: {}",
            pr_info.failing_checks.len(),
            pr_info.failing_checks.join(", ")
        );
        match ks
            .query(repo_path, Some(&worktree_path), &query_text, 5)
            .await
        {
            Ok(results) if !results.is_empty() => {
                #[allow(clippy::cast_possible_truncation)]
                {
                    knowledge_recalled = results.len() as u32;
                }
                for result in &results {
                    let summary = result.content.lines().next().unwrap_or(&result.content);
                    let _ = writeln!(knowledge_context, "- {summary}");
                }
                info!("Injected {knowledge_recalled} knowledge patterns into shepherd task");
            }
            Ok(_) => {
                debug!("No relevant knowledge patterns found for PR #{pr_number}");
            }
            Err(e) => {
                debug!("Knowledge query failed (non-fatal): {e}");
            }
        }
    }

    // 8. Generate task description with PR context + knowledge
    let description = generate_shepherd_prompt(
        &owner,
        &repo_name,
        &pr_info,
        if knowledge_context.is_empty() {
            None
        } else {
            Some(&knowledge_context)
        },
    );

    // 9. Create the task (clean up worktree on failure)
    let task_info = match task::create_task(
        &worktree_path,
        TaskType::PrShepherd,
        repo_path,
        branch,
        Some(pr_number),
        Some(&description),
    ) {
        Ok(info) => info,
        Err(e) => {
            error!("Task creation failed: {e}");
            // Only clean up worktree if we created it (don't delete pre-existing ones)
            if let Some(wt_name) = &created_worktree_name {
                if let Err(cleanup_err) = git::remove_worktree(repo, wt_name, false) {
                    error!("Failed to clean up orphaned worktree: {cleanup_err}");
                }
            }
            return Err(e);
        }
    };

    info!("Shepherded PR #{pr_number} for {owner}/{repo_name} → worktree at {worktree_path} (knowledge_recalled={knowledge_recalled})");

    Ok(ShepherdResult {
        task: task_info,
        worktree_path,
        knowledge_recalled,
    })
}

/// Result of a `shepherd_pr` operation — contains the task and worktree path for launch.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShepherdResult {
    pub task: TaskInfo,
    pub worktree_path: String,
    pub knowledge_recalled: u32,
}

/// Intermediate PR data extracted from octocrab for shepherd prompt generation.
struct PrContext {
    number: u64,
    title: String,
    branch: String,
    additions: Option<u64>,
    deletions: Option<u64>,
    changed_files: u64,
    failing_checks: Vec<String>,
    total_checks: usize,
    review_summary: String,
}

async fn find_pr_by_number(
    owner: &str,
    repo_name: &str,
    pr_number: u64,
) -> Result<PrContext, AppError> {
    let client = github::get_client_pub().await?;

    let pr = client
        .pulls(owner, repo_name)
        .get(pr_number)
        .await
        .map_err(|e| {
            error!("Failed to fetch PR #{pr_number} for {owner}/{repo_name}: {e}");
            AppError::GitHub(format!("PR #{pr_number} not found: {e}"))
        })?;

    let branch = pr.head.ref_field.clone();

    // Fetch check runs
    let sha = pr.head.sha.clone();
    let (failing_checks, total_checks) = match client
        .checks(owner, repo_name)
        .list_check_runs_for_git_ref(octocrab::params::repos::Commitish(sha))
        .send()
        .await
    {
        Ok(list) => {
            let failing: Vec<String> = list
                .check_runs
                .iter()
                .filter(|cr| {
                    cr.conclusion
                        .as_ref()
                        .is_some_and(|c| c != "success" && c != "skipped" && c != "neutral")
                })
                .map(|cr| cr.name.clone())
                .collect();
            let total = list.check_runs.len();
            (failing, total)
        }
        Err(e) => {
            error!("Failed to fetch checks for PR #{pr_number}: {e}");
            (Vec::new(), 0)
        }
    };

    // Fetch reviews
    let review_summary = match client
        .get::<Vec<octocrab::models::pulls::Review>, _, _>(
            format!("/repos/{owner}/{repo_name}/pulls/{pr_number}/reviews"),
            None::<&()>,
        )
        .await
    {
        Ok(reviews) => {
            if reviews.is_empty() {
                "No reviews".to_string()
            } else {
                reviews
                    .iter()
                    .filter_map(|r| {
                        let user = r.user.as_ref()?.login.clone();
                        let state = r.state.map(|s| format!("{s:?}")).unwrap_or_default();
                        Some(format!("{user}: {state}"))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        Err(e) => {
            error!("Failed to fetch reviews for PR #{pr_number}: {e}");
            "Unable to fetch reviews".to_string()
        }
    };

    Ok(PrContext {
        number: pr_number,
        title: pr.title.clone().unwrap_or_default(),
        branch,
        additions: pr.additions,
        deletions: pr.deletions,
        changed_files: pr.changed_files.unwrap_or(0),
        failing_checks,
        total_checks,
        review_summary,
    })
}

fn generate_shepherd_prompt(
    owner: &str,
    repo_name: &str,
    pr: &PrContext,
    knowledge_context: Option<&str>,
) -> String {
    let mut prompt = String::new();

    let _ = writeln!(prompt, "# Shepherd PR #{}: {}", pr.number, pr.title);
    let _ = writeln!(prompt);
    let _ = writeln!(prompt, "## Goal");
    let _ = writeln!(prompt, "Get this PR to green and ready for merge.");
    let _ = writeln!(prompt);
    let _ = writeln!(prompt, "## PR Context");
    let _ = writeln!(prompt, "- Branch: {}", pr.branch);
    let _ = writeln!(
        prompt,
        "- CI Status: {} checks — {} failing",
        pr.total_checks,
        pr.failing_checks.len()
    );
    if !pr.failing_checks.is_empty() {
        let _ = writeln!(prompt, "- Failing checks: {}", pr.failing_checks.join(", "));
    }
    let _ = writeln!(prompt, "- Reviews: {}", pr.review_summary);
    let _ = writeln!(
        prompt,
        "- Diff: +{} -{} across {} files",
        pr.additions.unwrap_or(0),
        pr.deletions.unwrap_or(0),
        pr.changed_files
    );
    let _ = writeln!(prompt);
    let _ = writeln!(prompt, "## Instructions");
    let _ = writeln!(
        prompt,
        "1. Read the failing CI logs: `gh pr checks {} --repo {owner}/{repo_name}`",
        pr.number
    );
    let _ = writeln!(
        prompt,
        "2. Read review comments: `gh pr view {} --repo {owner}/{repo_name} --comments`",
        pr.number
    );
    let _ = writeln!(prompt, "3. Fix the issues in the code");
    let _ = writeln!(
        prompt,
        "4. Stage only the files you changed, commit and push: `git add <files> && git commit -m \"fix: address CI failures and review comments\" && git push`"
    );
    let _ = writeln!(
        prompt,
        "5. Wait for CI: `gh pr checks {} --repo {owner}/{repo_name} --watch`",
        pr.number
    );
    let _ = writeln!(prompt, "6. If still failing, repeat from step 1");
    let _ = writeln!(
        prompt,
        "7. When all checks pass and reviews are addressed, report success"
    );
    let _ = writeln!(prompt);
    let _ = writeln!(prompt, "## Prior Knowledge");
    if let Some(context) = knowledge_context {
        let _ = write!(prompt, "{context}");
    } else {
        let _ = writeln!(prompt, "(none yet)");
    }

    prompt
}

/// Fetch a remote branch to local if it doesn't exist locally.
fn fetch_branch_if_needed(repo_path: &Path, branch: &str) -> Result<(), AppError> {
    let repo = git2::Repository::open(repo_path).map_err(|e| {
        error!("Failed to open repo for fetch: {e}");
        AppError::Git(e)
    })?;

    // Check if branch exists locally
    if repo.find_branch(branch, git2::BranchType::Local).is_ok() {
        return Ok(());
    }

    // Try to fetch from origin
    let mut remote = repo.find_remote("origin").map_err(|e| {
        error!("No origin remote for fetch: {e}");
        AppError::Git(e)
    })?;

    info!("Fetching branch {branch:?} from origin");
    remote.fetch(&[branch], None, None).map_err(|e| {
        error!("Failed to fetch branch {branch:?}: {e}");
        AppError::Git(e)
    })?;

    // Create local branch tracking the remote
    let remote_ref = format!("refs/remotes/origin/{branch}");
    let reference = repo.find_reference(&remote_ref).map_err(|e| {
        error!("Remote branch {remote_ref} not found after fetch: {e}");
        AppError::Git(e)
    })?;
    let commit = reference.peel_to_commit().map_err(|e| {
        error!("Failed to peel remote ref to commit: {e}");
        AppError::Git(e)
    })?;
    repo.branch(branch, &commit, false).map_err(|e| {
        error!("Failed to create local branch {branch:?}: {e}");
        AppError::Git(e)
    })?;

    info!("Fetched and created local branch {branch:?}");
    Ok(())
}
