use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::agent::{AgentState, FileAccess};
use log::debug;
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionActivity {
    pub agents: Vec<AgentState>,
    pub files: Vec<FileAccess>,
}

pub async fn get_session_activity(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Json<SessionActivity> {
    debug!("Fetching activity for session: {session_id}");
    let agents = state
        .activity_store
        .get_agents_for_session(&session_id)
        .await;
    let files = state
        .activity_store
        .get_files_for_session(&session_id)
        .await;

    Json(SessionActivity { agents, files })
}

pub async fn get_active_agents(State(state): State<AppState>) -> Json<Vec<AgentState>> {
    let agents = state.activity_store.get_active_sessions().await;
    Json(agents)
}
