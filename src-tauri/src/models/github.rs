use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PrSummary {
    pub number: u64,
    pub title: String,
    pub branch: String,
    pub url: String,
    pub ci_status: Option<String>,
    pub review_decision: Option<String>,
    pub repo_name: String,
    pub author: String,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrFilter {
    pub author: Option<String>,
    pub ci_status: Option<String>,
    pub label: Option<String>,
}
