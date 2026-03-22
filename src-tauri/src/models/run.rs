use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Created,
    Starting,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInfo {
    pub session_id: Option<String>,
    pub task_path: String,
    pub status: RunStatus,
    pub started_at: String,
    pub cost_usd: f64,
    #[serde(default)]
    pub last_heartbeat: Option<String>,
    #[serde(default)]
    pub elapsed_secs: u64,
    #[serde(default)]
    pub tab_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarRequest {
    LaunchRun {
        task_path: String,
        worktree: String,
        options: LaunchOptions,
        hook_port: u16,
        tab_id: String,
    },
    ResumeRun {
        task_path: String,
        worktree: String,
        session_id: String,
        options: LaunchOptions,
        hook_port: u16,
        tab_id: String,
    },
    CancelRun {
        session_id: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOptions {
    pub max_turns: Option<u32>,
    pub max_budget_usd: Option<f64>,
    #[serde(default)]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SidecarResponse {
    Heartbeat {
        session_id: Option<String>,
    },
    SessionStarted {
        session_id: String,
    },
    RunStep {
        step: String,
        detail: String,
        session_id: Option<String>,
    },
    AssistantText {
        text: String,
        session_id: Option<String>,
    },
    ToolCall {
        tool: String,
        file_path: Option<String>,
        session_id: Option<String>,
    },
    RunComplete {
        status: String,
        cost_usd: Option<f64>,
        session_id: Option<String>,
    },
    RunError {
        status: String,
        error: String,
        cost_usd: Option<f64>,
        session_id: Option<String>,
    },
    PermissionRequest {
        tool: Option<String>,
        command: Option<String>,
        tool_use_id: String,
        session_id: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionResponseMsg {
    PermissionResponse {
        tool_use_id: String,
        decision: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingPermission {
    pub tool: Option<String>,
    pub command: Option<String>,
    pub tool_use_id: String,
    pub requested_at: u64,
}
