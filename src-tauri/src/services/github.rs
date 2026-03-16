use crate::error::AppError;
use crate::models::PrInfo;
use log::{debug, error};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

type PrCache = Mutex<HashMap<String, (Instant, Option<PrInfo>)>>;

static PR_CACHE: std::sync::OnceLock<PrCache> = std::sync::OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, (Instant, Option<PrInfo>)>> {
    PR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const CACHE_TTL: Duration = Duration::from_secs(300);

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

    let token = resolve_github_token().await?;
    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token)
        .build()
        .map_err(|e| AppError::GitHub(e.to_string()))?;

    let pulls = octocrab
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

    let pr_info = pulls.items.first().map(|pr| {
        let state = if pr.merged_at.is_some() {
            "merged".to_string()
        } else {
            match pr.state {
                Some(octocrab::models::IssueState::Open) => "open".to_string(),
                Some(octocrab::models::IssueState::Closed) => "closed".to_string(),
                _ => "unknown".to_string(),
            }
        };

        PrInfo {
            number: pr.number,
            title: pr.title.clone().unwrap_or_default(),
            state,
            is_draft: pr.draft.unwrap_or(false),
            url: pr
                .html_url
                .as_ref()
                .map_or_else(String::new, std::string::ToString::to_string),
            ci_status: None,
        }
    });

    // Update cache
    if let Ok(mut c) = cache().lock() {
        c.insert(cache_key.clone(), (Instant::now(), pr_info.clone()));
    }

    debug!(
        "Fetched PR for {cache_key}: {:?}",
        pr_info.as_ref().map(|p| p.number)
    );

    Ok(pr_info)
}
