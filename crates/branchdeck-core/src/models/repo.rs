use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    pub name: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub path: PathBuf,
    pub current_branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
    pub name: String,
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub path: PathBuf,
    pub branch: String,
    pub is_main: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorktreePreview {
    pub sanitized_name: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
    pub base_branch: String,
    pub branch_exists: bool,
    pub path_exists: bool,
    pub worktree_exists: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub is_remote: bool,
    pub is_head: bool,
    pub has_worktree: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrackingInfo {
    pub ahead: usize,
    pub behind: usize,
    pub upstream_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckRunInfo {
    pub name: String,
    pub conclusion: Option<String>,
    pub status: String,
    pub details_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInfo {
    pub user: String,
    pub state: String,
    pub submitted_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub is_draft: bool,
    pub url: String,
    pub checks: Vec<CheckRunInfo>,
    pub reviews: Vec<ReviewInfo>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub review_decision: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_check_run_info_serde() {
        let info = CheckRunInfo {
            name: "ci/build".to_string(),
            conclusion: Some("success".to_string()),
            status: "completed".to_string(),
            details_url: Some("https://example.com/run/1".to_string()),
        };

        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("name").is_some());
        assert!(json.get("conclusion").is_some());
        assert!(json.get("status").is_some());
        assert!(json.get("detailsUrl").is_some());
        // Ensure snake_case keys are absent
        assert!(json.get("details_url").is_none());

        let roundtrip: CheckRunInfo = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip, info);
    }

    #[test]
    fn test_review_info_serde() {
        let info = ReviewInfo {
            user: "alice".to_string(),
            state: "approved".to_string(),
            submitted_at: Some("2026-01-15T10:00:00Z".to_string()),
        };

        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("user").is_some());
        assert!(json.get("state").is_some());
        assert!(json.get("submittedAt").is_some());
        assert!(json.get("submitted_at").is_none());

        let roundtrip: ReviewInfo = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip, info);
    }

    #[test]
    fn test_pr_info_serde() {
        let info = PrInfo {
            number: 42,
            title: "Add feature".to_string(),
            state: "open".to_string(),
            is_draft: false,
            url: "https://github.com/owner/repo/pull/42".to_string(),
            checks: vec![CheckRunInfo {
                name: "test".to_string(),
                conclusion: Some("success".to_string()),
                status: "completed".to_string(),
                details_url: None,
            }],
            reviews: vec![ReviewInfo {
                user: "bob".to_string(),
                state: "approved".to_string(),
                submitted_at: None,
            }],
            additions: Some(10),
            deletions: Some(3),
            review_decision: Some("approved".to_string()),
        };

        let json = serde_json::to_value(&info).unwrap();
        assert!(json.get("number").is_some());
        assert!(json.get("title").is_some());
        assert!(json.get("isDraft").is_some());
        assert!(json.get("reviewDecision").is_some());
        assert!(json.get("is_draft").is_none());
        assert!(json.get("review_decision").is_none());

        let roundtrip: PrInfo = serde_json::from_value(json).unwrap();
        assert_eq!(roundtrip, info);
    }
}
