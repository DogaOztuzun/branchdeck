use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeType {
    Trajectory,
    Commit,
    Explicit,
    ErrorResolution,
    Pattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeSource {
    EventBus,
    Mcp,
    User,
    GitHook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeMetadata {
    pub session_id: Option<String>,
    pub tool_names: Vec<String>,
    pub file_paths: Vec<String>,
    pub run_status: Option<String>,
    pub cost_usd: Option<f64>,
    pub quality_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntry {
    pub id: u64,
    pub content: String,
    pub entry_type: KnowledgeType,
    pub source: KnowledgeSource,
    pub repo_hash: String,
    pub worktree_id: Option<String>,
    pub metadata: KnowledgeMetadata,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeStats {
    pub total_entries: u64,
    pub trajectory_count: u64,
    pub explicit_count: u64,
    pub avg_quality: f32,
    pub last_ingested: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub id: u64,
    pub content: String,
    pub entry_type: KnowledgeType,
    pub distance: f32,
    pub metadata: KnowledgeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub promoted: u64,
    pub discarded: u64,
    pub deleted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryRecord {
    pub session_id: String,
    pub tab_id: String,
    pub steps: Vec<TrajectoryStep>,
    pub quality_score: f32,
    pub started_at: u64,
    pub ended_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrajectoryStep {
    pub tool_name: String,
    pub file_path: Option<String>,
    pub was_modified: bool,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub id: u64,
    pub content: String,
    pub distance: f32,
    pub avg_quality: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SonaStats {
    pub trajectories_buffered: usize,
    pub patterns_stored: usize,
    pub patterns_persisted: u64,
}

/// Pending entry waiting for ONNX embedder to become available.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingEntry {
    pub content: String,
    pub entry_type: KnowledgeType,
    pub source: KnowledgeSource,
    pub repo_hash: String,
    pub worktree_id: Option<String>,
    pub metadata: KnowledgeMetadata,
    pub created_at: u64,
}
