use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Created,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskType {
    IssueFix,
    PrShepherd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskScope {
    Worktree,
    Workspace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TaskFrontmatter {
    #[serde(rename = "type")]
    pub task_type: TaskType,
    pub scope: TaskScope,
    pub status: TaskStatus,
    pub repo: String,
    pub branch: String,
    pub pr: Option<u64>,
    pub created: String,
    pub run_count: u32,
    /// Current SAT fix-verify cycle iteration (1-based). `None` if not in a cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_iteration: Option<u32>,
    /// Maximum SAT fix-verify cycle iterations allowed. `None` if not in a cycle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_max: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
    pub frontmatter: TaskFrontmatter,
    pub body: String,
    pub path: String,
}
