use serde::{Deserialize, Serialize};

use super::github::PrSummary;

pub type EpochMs = u64;

#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_ms() -> EpochMs {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    SessionStart {
        session_id: String,
        tab_id: String,
        model: Option<String>,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    ToolStart {
        session_id: String,
        agent_id: Option<String>,
        tab_id: String,
        tool_name: String,
        tool_use_id: String,
        file_path: Option<String>,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    ToolEnd {
        session_id: String,
        agent_id: Option<String>,
        tab_id: String,
        tool_name: String,
        tool_use_id: String,
        file_path: Option<String>,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    SubagentStart {
        session_id: String,
        agent_id: String,
        agent_type: String,
        tab_id: String,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    SubagentStop {
        session_id: String,
        agent_id: String,
        agent_type: String,
        tab_id: String,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    SessionStop {
        session_id: String,
        tab_id: String,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    Notification {
        session_id: String,
        tab_id: String,
        title: Option<String>,
        message: String,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    RunComplete {
        session_id: String,
        tab_id: String,
        status: String,
        cost_usd: f64,
        elapsed_secs: u64,
        ts: EpochMs,
    },
    #[serde(rename_all = "camelCase")]
    PrStatusChanged {
        repo: String,
        prs: Vec<PrSummary>,
        ts: EpochMs,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    Active,
    Idle,
    Stopped,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentState {
    pub session_id: String,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub tab_id: String,
    pub status: AgentStatus,
    pub current_tool: Option<String>,
    pub current_file: Option<String>,
    pub started_at: EpochMs,
    pub last_activity: EpochMs,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileAccess {
    pub path: String,
    pub last_tool: String,
    pub last_agent: String,
    pub last_access: EpochMs,
    pub access_count: u32,
    pub was_modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub permission_mode: Option<String>,
    pub file_path: String,
}

#[derive(Debug, Deserialize)]
pub struct HookPayload {
    pub session_id: String,
    pub hook_event_name: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub agent_type: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}
