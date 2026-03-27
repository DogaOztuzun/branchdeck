use crate::error::AppError;
use crate::models::github::{IssueSummary, PrFilter, PrSummary};
use crate::models::{CheckRunInfo, PrInfo, ReviewInfo};
use git2::Repository;
use log::{debug, error, info};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type PrCache = Mutex<HashMap<String, (Instant, Option<PrInfo>)>>;

static PR_CACHE: std::sync::OnceLock<PrCache> = std::sync::OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, (Instant, Option<PrInfo>)>> {
    PR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

static CLIENT_CACHE: std::sync::OnceLock<Mutex<Option<(Instant, octocrab::Octocrab)>>> =
    std::sync::OnceLock::new();

fn client_cache() -> &'static Mutex<Option<(Instant, octocrab::Octocrab)>> {
    CLIENT_CACHE.get_or_init(|| Mutex::new(None))
}

const CACHE_TTL: Duration = Duration::from_secs(30);
const CLIENT_TTL: Duration = Duration::from_secs(300);

/// Get (or create) the cached octocrab client. Public for sibling services.
///
/// # Errors
/// Returns `AppError` if the GitHub token cannot be resolved or the client cannot be built.
pub async fn get_client_pub() -> Result<octocrab::Octocrab, AppError> {
    get_client().await
}

async fn get_client() -> Result<octocrab::Octocrab, AppError> {
    // Check for a cached client (brief lock, no await)
    if let Ok(guard) = client_cache().lock() {
        if let Some((ts, ref client)) = *guard {
            if ts.elapsed() < CLIENT_TTL {
                debug!("Using cached octocrab client");
                return Ok(client.clone());
            }
        }
    }

    let token = resolve_github_token().await?;
    let client = octocrab::Octocrab::builder()
        .personal_token(token)
        .build()
        .map_err(|e| {
            error!("Failed to build octocrab client: {e}");
            AppError::GitHub(e.to_string())
        })?;

    // Cache the new client (brief lock, no await)
    if let Ok(mut guard) = client_cache().lock() {
        *guard = Some((Instant::now(), client.clone()));
    }

    info!("Created new octocrab client");
    Ok(client)
}

fn infer_check_status_from_fields(
    conclusion: Option<&String>,
    started_at: Option<&chrono::DateTime<chrono::Utc>>,
) -> String {
    if conclusion.is_some() {
        "completed".to_string()
    } else if started_at.is_some() {
        "in_progress".to_string()
    } else {
        "queued".to_string()
    }
}

fn infer_check_status(check_run: &octocrab::models::checks::CheckRun) -> String {
    infer_check_status_from_fields(check_run.conclusion.as_ref(), check_run.started_at.as_ref())
}

fn review_state_to_string(state: octocrab::models::pulls::ReviewState) -> String {
    use octocrab::models::pulls::ReviewState;
    match state {
        ReviewState::Approved => "approved".to_string(),
        ReviewState::ChangesRequested => "changes_requested".to_string(),
        ReviewState::Commented => "commented".to_string(),
        ReviewState::Dismissed => "dismissed".to_string(),
        ReviewState::Pending => "pending".to_string(),
        ReviewState::Open => "open".to_string(),
        _ => "unknown".to_string(),
    }
}

fn derive_review_decision(reviews: &[ReviewInfo]) -> Option<String> {
    // Group by user, keep latest per user (by submitted_at)
    let mut latest_by_user: HashMap<&str, &ReviewInfo> = HashMap::new();
    for review in reviews {
        let dominated = latest_by_user
            .get(review.user.as_str())
            .is_some_and(
                |existing| match (&existing.submitted_at, &review.submitted_at) {
                    (Some(a), Some(b)) => b > a,
                    (None, Some(_)) => true,
                    _ => false,
                },
            );
        if dominated || !latest_by_user.contains_key(review.user.as_str()) {
            latest_by_user.insert(&review.user, review);
        }
    }

    // Filter out "commented" and "dismissed"
    let meaningful: Vec<&str> = latest_by_user
        .values()
        .map(|r| r.state.as_str())
        .filter(|s| *s != "commented" && *s != "dismissed")
        .collect();

    if meaningful.contains(&"changes_requested") {
        Some("changes_requested".to_string())
    } else if meaningful.contains(&"approved") {
        Some("approved".to_string())
    } else {
        None
    }
}

#[must_use]
pub fn parse_github_remote(remote_url: &str) -> Option<(String, String)> {
    // SSH: git@github.com:owner/repo.git
    if let Some(rest) = remote_url.strip_prefix("git@github.com:") {
        let clean = rest.trim_end_matches(".git");
        let parts: Vec<&str> = clean.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    // HTTPS: https://github.com/owner/repo.git
    if remote_url.contains("github.com/") {
        let after = remote_url.split("github.com/").nth(1)?;
        let clean = after.trim_end_matches(".git");
        let parts: Vec<&str> = clean.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }
    None
}

/// # Errors
/// Returns `AppError` if the `gh` CLI is not available or authentication fails.
pub async fn resolve_github_token() -> Result<String, AppError> {
    let output = tokio::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .await
        .map_err(|e| {
            debug!("gh CLI not available: {e}");
            AppError::GitHub(format!("gh CLI not available: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("gh auth token failed: {stderr}");
        return Err(AppError::GitHub(format!("gh auth failed: {stderr}")));
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return Err(AppError::GitHub("Empty token from gh auth".to_string()));
    }
    Ok(token)
}

/// # Errors
/// Returns `AppError` if authentication or the GitHub API request fails.
#[allow(clippy::too_many_lines)]
pub async fn get_pr_for_branch(
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<PrInfo>, AppError> {
    let cache_key = format!("{owner}/{repo}:{branch}");

    // Check cache
    if let Ok(c) = cache().lock() {
        if let Some((ts, cached)) = c.get(&cache_key) {
            if ts.elapsed() < CACHE_TTL {
                debug!("PR cache hit for {cache_key}");
                return Ok(cached.clone());
            }
        }
    }

    let client = get_client().await?;

    let pulls = client
        .pulls(owner, repo)
        .list()
        .head(format!("{owner}:{branch}"))
        .state(octocrab::params::State::All)
        .per_page(1)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to fetch PRs for {cache_key}: {e}");
            AppError::GitHub(e.to_string())
        })?;

    let Some(pr) = pulls.items.first() else {
        // No PR found — cache the None result
        if let Ok(mut c) = cache().lock() {
            c.insert(cache_key.clone(), (Instant::now(), None));
        }
        debug!("No PR found for {cache_key}");
        return Ok(None);
    };

    let state = if pr.merged_at.is_some() {
        "merged".to_string()
    } else {
        match pr.state {
            Some(octocrab::models::IssueState::Open) => "open".to_string(),
            Some(octocrab::models::IssueState::Closed) => "closed".to_string(),
            _ => "unknown".to_string(),
        }
    };

    // Fetch check runs for the head SHA
    let sha = pr.head.sha.clone();
    let checks = match client
        .checks(owner, repo)
        .list_check_runs_for_git_ref(octocrab::params::repos::Commitish(sha.clone()))
        .send()
        .await
    {
        Ok(list) => {
            debug!(
                "Fetched {} check runs for {cache_key} (sha {sha})",
                list.check_runs.len()
            );
            // Deduplicate by name — GitHub returns multiple runs for re-runs;
            // keep only the latest (first seen, since API returns newest first)
            let mut seen = std::collections::HashSet::new();
            list.check_runs
                .iter()
                .filter(|cr| seen.insert(cr.name.clone()))
                .map(|cr| CheckRunInfo {
                    name: cr.name.clone(),
                    conclusion: cr.conclusion.clone(),
                    status: infer_check_status(cr),
                    details_url: cr.details_url.clone(),
                })
                .collect::<Vec<_>>()
        }
        Err(e) => {
            error!("Failed to fetch check runs for {cache_key}: {e}");
            Vec::new()
        }
    };

    // Fetch reviews
    let reviews: Vec<ReviewInfo> = match client
        .get::<Vec<octocrab::models::pulls::Review>, _, _>(
            format!("/repos/{owner}/{repo}/pulls/{}/reviews", pr.number),
            None::<&()>,
        )
        .await
    {
        Ok(review_list) => {
            debug!("Fetched {} reviews for {cache_key}", review_list.len());
            review_list
                .iter()
                .map(|r| ReviewInfo {
                    user: r.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
                    state: r
                        .state
                        .map_or_else(|| "pending".to_string(), review_state_to_string),
                    submitted_at: r.submitted_at.map(|t| t.to_rfc3339()),
                })
                .collect()
        }
        Err(e) => {
            error!("Failed to fetch reviews for {cache_key}: {e}");
            Vec::new()
        }
    };

    let review_decision = derive_review_decision(&reviews);

    let pr_info = PrInfo {
        number: pr.number,
        title: pr.title.clone().unwrap_or_default(),
        state,
        is_draft: pr.draft.unwrap_or(false),
        url: pr
            .html_url
            .as_ref()
            .map_or_else(String::new, std::string::ToString::to_string),
        checks,
        reviews,
        additions: pr.additions,
        deletions: pr.deletions,
        review_decision,
    };

    // Update cache
    if let Ok(mut c) = cache().lock() {
        c.insert(cache_key.clone(), (Instant::now(), Some(pr_info.clone())));
    }

    info!(
        "Fetched PR #{} for {cache_key} (checks={}, reviews={})",
        pr_info.number,
        pr_info.checks.len(),
        pr_info.reviews.len()
    );

    Ok(Some(pr_info))
}

/// Resolve owner/repo from a local git repository's origin remote.
///
/// # Errors
/// Returns `AppError` if the repo cannot be opened or has no GitHub remote.
pub fn resolve_owner_repo(repo_path: &Path) -> Result<(String, String), AppError> {
    let repo = Repository::open(repo_path).map_err(|e| {
        error!("Failed to open repo at {}: {e}", repo_path.display());
        AppError::Git(e)
    })?;

    let remote = repo.find_remote("origin").map_err(|e| {
        debug!("No origin remote for {}: {e}", repo_path.display());
        AppError::GitHub(format!(
            "No GitHub remote configured for {}",
            repo_path.display()
        ))
    })?;

    let remote_url = remote.url().unwrap_or("").to_string();
    parse_github_remote(&remote_url)
        .ok_or_else(|| AppError::GitHub(format!("Not a GitHub remote: {remote_url}")))
}

/// List all open PRs for a GitHub repository.
///
/// # Errors
/// Returns `AppError` on authentication, API, or rate-limit failures.
#[allow(clippy::too_many_lines)]
pub async fn list_open_prs(
    repo_path: &str,
    filter: Option<PrFilter>,
) -> Result<Vec<PrSummary>, AppError> {
    let (owner, repo_name) = resolve_owner_repo(Path::new(repo_path))?;
    let display_name = format!("{owner}/{repo_name}");

    let client = get_client().await?;

    let pulls = client
        .pulls(&owner, &repo_name)
        .list()
        .state(octocrab::params::State::Open)
        .per_page(100)
        .send()
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("rate limit") || msg.contains("403") {
                error!("GitHub rate limit for {owner}/{repo_name}: {e}");
                AppError::GitHub(format!(
                    "GitHub rate limit exceeded for {owner}/{repo_name}"
                ))
            } else if msg.contains("401") || msg.contains("Bad credentials") {
                error!("GitHub auth error for {owner}/{repo_name}: {e}");
                AppError::GitHub("GitHub authentication failed — check your token".to_string())
            } else {
                error!("Failed to list PRs for {owner}/{repo_name}: {e}");
                AppError::GitHub(e.to_string())
            }
        })?;

    let filter = filter.unwrap_or_default();

    let summaries: Vec<PrSummary> = pulls
        .items
        .iter()
        .filter_map(|pr| {
            let author = pr
                .user
                .as_ref()
                .map(|u| u.login.clone())
                .unwrap_or_default();

            // Apply author filter
            if let Some(ref filter_author) = filter.author {
                if !author.eq_ignore_ascii_case(filter_author) {
                    return None;
                }
            }

            // CI status and review decision require enrichment (separate API calls).
            // Set to None here — frontend enriches lazily after load.
            let ci_status: Option<String> = None;
            let review_decision: Option<String> = None;

            Some(PrSummary {
                number: pr.number,
                title: pr.title.clone().unwrap_or_default(),
                branch: pr.head.ref_field.clone(),
                base_branch: pr.base.ref_field.clone(),
                url: pr
                    .html_url
                    .as_ref()
                    .map_or_else(String::new, std::string::ToString::to_string),
                ci_status,
                review_decision,
                repo_name: display_name.clone(),
                author,
                additions: pr.additions,
                deletions: pr.deletions,
                changed_files: pr.changed_files,
                created_at: pr.created_at.map(|t| t.to_rfc3339()),
                head_sha: Some(pr.head.sha.clone()),
            })
        })
        .collect();

    info!(
        "Listed {} open PRs for {owner}/{repo_name}",
        summaries.len()
    );
    Ok(summaries)
}

/// List open PRs across multiple repositories.
///
/// # Errors
/// Returns errors per-repo but continues collecting from other repos.
pub async fn list_all_open_prs(
    repo_paths: &[String],
    filter: Option<PrFilter>,
) -> Result<Vec<PrSummary>, AppError> {
    let mut all_prs = Vec::new();
    let mut last_error: Option<AppError> = None;
    let mut any_succeeded = false;

    for repo_path in repo_paths {
        match list_open_prs(repo_path, filter.clone()).await {
            Ok(prs) => {
                any_succeeded = true;
                all_prs.extend(prs);
            }
            Err(e) => {
                error!("Failed to list PRs for {repo_path}: {e}");
                last_error = Some(e);
            }
        }
    }

    // Only return error if ALL repos failed (not just empty results)
    if !any_succeeded {
        if let Some(e) = last_error {
            return Err(e);
        }
    }

    info!(
        "Listed {} total open PRs across {} repos",
        all_prs.len(),
        repo_paths.len()
    );
    Ok(all_prs)
}

/// List recently merged PRs for a single repo.
/// Queries closed PRs and filters to those with `merged_at` set.
/// Returns the 20 most recent closed PRs that were actually merged.
///
/// # Errors
/// Returns `AppError` on authentication, API, or rate-limit failures.
pub async fn list_recently_merged_prs(
    repo_path: &str,
) -> Result<Vec<crate::models::github::MergedPrInfo>, AppError> {
    let (owner, repo_name) = resolve_owner_repo(Path::new(repo_path))?;
    let display_name = format!("{owner}/{repo_name}");

    let client = get_client().await?;

    let pulls = client
        .pulls(&owner, &repo_name)
        .list()
        .state(octocrab::params::State::Closed)
        .sort(octocrab::params::pulls::Sort::Updated)
        .direction(octocrab::params::Direction::Descending)
        .per_page(20)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to list merged PRs for {owner}/{repo_name}: {e}");
            AppError::GitHub(e.to_string())
        })?;

    let merged: Vec<crate::models::github::MergedPrInfo> = pulls
        .items
        .iter()
        .filter_map(|pr| {
            let merged_at = pr.merged_at?;
            Some(crate::models::github::MergedPrInfo {
                number: pr.number,
                title: pr.title.clone().unwrap_or_default(),
                branch: pr.head.ref_field.clone(),
                base_branch: pr.base.ref_field.clone(),
                repo_name: display_name.clone(),
                merged_at: merged_at.to_rfc3339(),
            })
        })
        .collect();

    debug!(
        "Found {} recently merged PRs for {display_name}",
        merged.len()
    );
    Ok(merged)
}

/// List recently merged PRs across all managed repos.
///
/// # Errors
/// Returns `AppError` only if ALL repos fail.
pub async fn list_all_recently_merged_prs(
    repo_paths: &[String],
) -> Result<Vec<crate::models::github::MergedPrInfo>, AppError> {
    let mut all_merged = Vec::new();
    let mut last_error: Option<AppError> = None;
    let mut any_succeeded = false;

    for repo_path in repo_paths {
        match list_recently_merged_prs(repo_path).await {
            Ok(merged) => {
                any_succeeded = true;
                all_merged.extend(merged);
            }
            Err(e) => {
                error!("Failed to list merged PRs for {repo_path}: {e}");
                last_error = Some(e);
            }
        }
    }

    if !any_succeeded {
        if let Some(e) = last_error {
            return Err(e);
        }
    }

    info!(
        "Listed {} total merged PRs across {} repos",
        all_merged.len(),
        repo_paths.len()
    );
    Ok(all_merged)
}

/// Enrich a `PrSummary` with CI status and review decision by fetching checks and reviews.
///
/// # Errors
/// Returns `AppError` on API failures.
pub async fn enrich_pr_summary(repo_path: &str, pr: &mut PrSummary) -> Result<(), AppError> {
    let (owner, repo_name) = resolve_owner_repo(Path::new(repo_path))?;

    // Fetch the full PR info using existing function
    let pr_info = get_pr_for_branch(&owner, &repo_name, &pr.branch).await?;

    if let Some(info) = pr_info {
        // Derive CI status from check conclusions
        let has_failing = info.checks.iter().any(|c| {
            c.conclusion
                .as_ref()
                .is_some_and(|conc| conc != "success" && conc != "skipped" && conc != "neutral")
        });
        let has_pending = info.checks.iter().any(|c| c.status != "completed");

        pr.ci_status = Some(if has_failing {
            "failing".to_string()
        } else if has_pending {
            "pending".to_string()
        } else {
            "passing".to_string()
        });

        pr.review_decision = info.review_decision;
    }

    Ok(())
}

/// List open issues with a specific label for a single repo.
///
/// # Errors
/// Returns `AppError` on API or git failures.
pub async fn list_issues_with_label(
    repo_path: &str,
    label: &str,
) -> Result<Vec<IssueSummary>, AppError> {
    let (owner, repo_name) = resolve_owner_repo(Path::new(repo_path))?;
    let client = get_client().await?;
    let full_name = format!("{owner}/{repo_name}");

    let page = client
        .issues(&owner, &repo_name)
        .list()
        .labels(&[label.to_string()])
        .state(octocrab::params::State::Open)
        .per_page(30)
        .send()
        .await
        .map_err(|e| {
            error!("Failed to list issues for {full_name} with label {label:?}: {e}");
            AppError::GitHub(format!("list issues failed for {full_name}: {e}"))
        })?;

    let issues: Vec<IssueSummary> = page
        .items
        .into_iter()
        .filter(|issue| issue.pull_request.is_none()) // Exclude PRs (GitHub API returns PRs as issues)
        .map(|issue| IssueSummary {
            number: issue.number,
            title: issue.title,
            body: issue.body,
            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
            author: issue.user.login.clone(),
            repo_name: full_name.clone(),
            created_at: Some(issue.created_at.to_rfc3339()),
            url: issue.html_url.to_string(),
        })
        .collect();

    debug!(
        "Listed {} open issues with label {label:?} for {full_name}",
        issues.len()
    );
    Ok(issues)
}

/// List open issues with a specific label across all managed repos.
///
/// # Errors
/// Returns `AppError` only if ALL repos fail.
pub async fn list_all_issues_with_label(
    repo_paths: &[String],
    label: &str,
) -> Result<Vec<IssueSummary>, AppError> {
    let mut all_issues = Vec::new();
    let mut last_error: Option<AppError> = None;
    let mut any_succeeded = false;

    for repo_path in repo_paths {
        match list_issues_with_label(repo_path, label).await {
            Ok(issues) => {
                any_succeeded = true;
                all_issues.extend(issues);
            }
            Err(e) => {
                error!("Failed to list issues for {repo_path}: {e}");
                last_error = Some(e);
            }
        }
    }

    if !any_succeeded {
        if let Some(e) = last_error {
            return Err(e);
        }
    }

    info!(
        "Listed {} total open labeled issues across {} repos",
        all_issues.len(),
        repo_paths.len()
    );
    Ok(all_issues)
}

/// Add labels to a GitHub issue.
///
/// Uses the octocrab client to apply one or more labels to the specified issue.
/// This is non-fatal by design — callers should handle errors gracefully since
/// local persistence of the FP record is the primary concern.
///
/// # Errors
/// Returns `AppError::GitHub` if the API call fails.
pub async fn add_labels_to_issue(
    owner: &str,
    repo: &str,
    issue_number: u64,
    labels: &[String],
) -> Result<(), AppError> {
    let client = get_client_pub().await?;
    client
        .issues(owner, repo)
        .add_labels(issue_number, labels)
        .await
        .map_err(|e| {
            error!("Failed to add labels {labels:?} to {owner}/{repo}#{issue_number}: {e}");
            AppError::GitHub(format!(
                "failed to add labels to {owner}/{repo}#{issue_number}: {e}"
            ))
        })?;

    info!("Added labels {labels:?} to {owner}/{repo}#{issue_number}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReviewInfo;

    #[test]
    fn test_parse_github_remote_ssh() {
        let result = parse_github_remote("git@github.com:owner/repo.git");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_remote_https() {
        let result = parse_github_remote("https://github.com/owner/repo.git");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_remote_https_no_suffix() {
        let result = parse_github_remote("https://github.com/owner/repo");
        assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
    }

    #[test]
    fn test_parse_github_remote_non_github() {
        let result = parse_github_remote("https://gitlab.com/owner/repo.git");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_github_remote_invalid() {
        let result = parse_github_remote("not-a-url");
        assert_eq!(result, None);
    }

    #[test]
    fn test_derive_review_decision_single_approved() {
        let reviews = vec![ReviewInfo {
            user: "alice".to_string(),
            state: "approved".to_string(),
            submitted_at: Some("2026-01-15T10:00:00Z".to_string()),
        }];
        assert_eq!(
            derive_review_decision(&reviews),
            Some("approved".to_string())
        );
    }

    #[test]
    fn test_derive_review_decision_changes_requested_wins() {
        let reviews = vec![
            ReviewInfo {
                user: "alice".to_string(),
                state: "approved".to_string(),
                submitted_at: Some("2026-01-15T10:00:00Z".to_string()),
            },
            ReviewInfo {
                user: "bob".to_string(),
                state: "changes_requested".to_string(),
                submitted_at: Some("2026-01-15T11:00:00Z".to_string()),
            },
        ];
        assert_eq!(
            derive_review_decision(&reviews),
            Some("changes_requested".to_string())
        );
    }

    #[test]
    fn test_derive_review_decision_only_commented() {
        let reviews = vec![ReviewInfo {
            user: "alice".to_string(),
            state: "commented".to_string(),
            submitted_at: Some("2026-01-15T10:00:00Z".to_string()),
        }];
        assert_eq!(derive_review_decision(&reviews), None);
    }

    #[test]
    fn test_derive_review_decision_empty() {
        let reviews: Vec<ReviewInfo> = vec![];
        assert_eq!(derive_review_decision(&reviews), None);
    }

    #[test]
    fn test_review_state_to_string_maps_correctly() {
        use octocrab::models::pulls::ReviewState;
        assert_eq!(review_state_to_string(ReviewState::Approved), "approved");
        assert_eq!(
            review_state_to_string(ReviewState::ChangesRequested),
            "changes_requested"
        );
        assert_eq!(review_state_to_string(ReviewState::Commented), "commented");
        assert_eq!(review_state_to_string(ReviewState::Dismissed), "dismissed");
        assert_eq!(review_state_to_string(ReviewState::Pending), "pending");
    }

    #[test]
    fn test_infer_check_status_completed() {
        let conclusion = Some("success".to_string());
        let started_at = Some(chrono::Utc::now());
        assert_eq!(
            infer_check_status_from_fields(conclusion.as_ref(), started_at.as_ref()),
            "completed"
        );
    }

    #[test]
    fn test_infer_check_status_in_progress() {
        let conclusion = None;
        let started_at = Some(chrono::Utc::now());
        assert_eq!(
            infer_check_status_from_fields(conclusion.as_ref(), started_at.as_ref()),
            "in_progress"
        );
    }

    #[test]
    fn test_infer_check_status_queued() {
        let conclusion = None;
        let started_at = None;
        assert_eq!(
            infer_check_status_from_fields(conclusion.as_ref(), started_at.as_ref()),
            "queued"
        );
    }

    #[test]
    fn test_infer_check_status_completed_failure() {
        let conclusion = Some("failure".to_string());
        let started_at = None;
        assert_eq!(
            infer_check_status_from_fields(conclusion.as_ref(), started_at.as_ref()),
            "completed"
        );
    }

    #[test]
    fn test_derive_review_decision_latest_per_user() {
        let reviews = vec![
            ReviewInfo {
                user: "alice".to_string(),
                state: "changes_requested".to_string(),
                submitted_at: Some("2026-01-15T10:00:00Z".to_string()),
            },
            ReviewInfo {
                user: "alice".to_string(),
                state: "approved".to_string(),
                submitted_at: Some("2026-01-15T12:00:00Z".to_string()),
            },
        ];
        assert_eq!(
            derive_review_decision(&reviews),
            Some("approved".to_string())
        );
    }
}
