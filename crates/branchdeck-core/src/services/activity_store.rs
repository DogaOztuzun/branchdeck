use crate::models::agent::{now_ms, AgentState, AgentStatus, EpochMs, Event, FileAccess};
use crate::services::event_bus::EventBus;
use log::{debug, warn};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct StoreInner {
    agents: HashMap<String, AgentState>,
    files: HashMap<String, FileAccess>,
}

impl StoreInner {
    fn on_session_start(&mut self, session_id: String, tab_id: String, ts: EpochMs) {
        debug!("Session started: {session_id}");
        self.agents.insert(
            session_id.clone(),
            AgentState {
                session_id,
                agent_id: None,
                agent_type: None,
                tab_id,
                status: AgentStatus::Active,
                current_tool: None,
                current_file: None,
                started_at: ts,
                last_activity: ts,
            },
        );
    }

    fn on_tool_start(
        &mut self,
        session_id: &str,
        agent_id: Option<&str>,
        tool_name: &str,
        file_path: Option<&str>,
        ts: EpochMs,
    ) {
        let key = agent_key(session_id, agent_id);
        if let Some(agent) = self.agents.get_mut(&key) {
            agent.current_tool = Some(tool_name.to_owned());
            agent.current_file = file_path.map(str::to_owned);
            agent.status = AgentStatus::Active;
            agent.last_activity = ts;
        }
        if let Some(path) = file_path {
            upsert_file_access(&mut self.files, path, tool_name, &key, ts);
        }
    }

    fn on_tool_end(
        &mut self,
        session_id: &str,
        agent_id: Option<&str>,
        tool_name: &str,
        file_path: Option<&str>,
        ts: EpochMs,
    ) {
        let key = agent_key(session_id, agent_id);
        if let Some(agent) = self.agents.get_mut(&key) {
            agent.current_tool = None;
            agent.current_file = None;
            agent.status = AgentStatus::Idle;
            agent.last_activity = ts;
        }
        if let Some(path) = file_path {
            upsert_file_access(&mut self.files, path, tool_name, &key, ts);
            if tool_name == "Write" || tool_name == "Edit" {
                if let Some(file) = self.files.get_mut(path) {
                    file.was_modified = true;
                }
            }
        }
    }

    fn on_subagent_start(
        &mut self,
        session_id: String,
        agent_id: String,
        agent_type: String,
        tab_id: String,
        ts: EpochMs,
    ) {
        let key = format!("{session_id}:{agent_id}");
        debug!("Subagent started: {key}");
        self.agents.insert(
            key,
            AgentState {
                session_id,
                agent_id: Some(agent_id),
                agent_type: Some(agent_type),
                tab_id,
                status: AgentStatus::Active,
                current_tool: None,
                current_file: None,
                started_at: ts,
                last_activity: ts,
            },
        );
    }

    fn on_subagent_stop(&mut self, session_id: &str, agent_id: &str, ts: EpochMs) {
        let key = format!("{session_id}:{agent_id}");
        if let Some(agent) = self.agents.get_mut(&key) {
            agent.status = AgentStatus::Stopped;
            agent.last_activity = ts;
            debug!("Subagent stopped: {key}");
        }
    }

    fn on_session_stop(&mut self, session_id: &str, ts: EpochMs) {
        if let Some(agent) = self.agents.get_mut(session_id) {
            agent.status = AgentStatus::Stopped;
            agent.last_activity = ts;
            debug!("Session stopped: {session_id}");
        }
    }
}

pub struct ActivityStore {
    inner: Arc<Mutex<StoreInner>>,
}

impl Default for ActivityStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityStore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                agents: HashMap::new(),
                files: HashMap::new(),
            })),
        }
    }

    pub fn start_subscriber(self: &Arc<Self>, event_bus: &EventBus) {
        let store = Arc::clone(self);
        let mut rx = event_bus.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => store.handle_event(event).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("ActivityStore subscriber lagged, missed {n} events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("EventBus closed, stopping ActivityStore subscriber");
                        break;
                    }
                }
            }
        });
    }

    pub fn start_gc(self: &Arc<Self>, ttl_ms: EpochMs) {
        let store = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let cutoff = now_ms().saturating_sub(ttl_ms);
                let mut inner = store.inner.lock().await;
                let agent_before = inner.agents.len();
                inner.agents.retain(|_, a| a.last_activity >= cutoff);
                let agent_removed = agent_before - inner.agents.len();

                let file_before = inner.files.len();
                inner.files.retain(|_, f| f.last_access >= cutoff);
                let file_removed = file_before - inner.files.len();

                if agent_removed > 0 || file_removed > 0 {
                    debug!(
                        "GC removed {agent_removed} agents and {file_removed} file entries (ttl={ttl_ms}ms)"
                    );
                }
            }
        });
    }

    async fn handle_event(&self, event: Event) {
        let mut inner = self.inner.lock().await;
        match event {
            Event::SessionStart {
                session_id,
                tab_id,
                ts,
                ..
            } => inner.on_session_start(session_id, tab_id, ts),
            Event::ToolStart {
                session_id,
                agent_id,
                tool_name,
                file_path,
                ts,
                ..
            } => inner.on_tool_start(
                &session_id,
                agent_id.as_deref(),
                &tool_name,
                file_path.as_deref(),
                ts,
            ),
            Event::ToolEnd {
                session_id,
                agent_id,
                tool_name,
                file_path,
                ts,
                ..
            } => inner.on_tool_end(
                &session_id,
                agent_id.as_deref(),
                &tool_name,
                file_path.as_deref(),
                ts,
            ),
            Event::SubagentStart {
                session_id,
                agent_id,
                agent_type,
                tab_id,
                ts,
            } => inner.on_subagent_start(session_id, agent_id, agent_type, tab_id, ts),
            Event::SubagentStop {
                session_id,
                agent_id,
                ts,
                ..
            } => inner.on_subagent_stop(&session_id, &agent_id, ts),
            Event::SessionStop { session_id, ts, .. } => inner.on_session_stop(&session_id, ts),
            Event::Notification { session_id, ts, .. } => {
                if let Some(agent) = inner.agents.get_mut(&session_id) {
                    agent.last_activity = ts;
                }
            }
            Event::RunComplete { .. }
            | Event::PrStatusChanged { .. }
            | Event::RetryDue { .. }
            | Event::IssueDetected { .. }
            | Event::PrMerged { .. } => {
                // RunComplete handled by KnowledgeService
                // PrStatusChanged, RetryDue, IssueDetected, PrMerged handled by Orchestrator
            }
        }
    }

    pub async fn get_all_files(&self) -> Vec<FileAccess> {
        self.inner.lock().await.files.values().cloned().collect()
    }

    pub async fn get_agents_for_tab(&self, tab_id: &str) -> Vec<AgentState> {
        self.inner
            .lock()
            .await
            .agents
            .values()
            .filter(|a| a.tab_id == tab_id)
            .cloned()
            .collect()
    }

    pub async fn get_agents_for_session(&self, session_id: &str) -> Vec<AgentState> {
        self.inner
            .lock()
            .await
            .agents
            .values()
            .filter(|a| a.session_id == session_id)
            .cloned()
            .collect()
    }

    pub async fn get_files_for_session(&self, session_id: &str) -> Vec<FileAccess> {
        self.inner
            .lock()
            .await
            .files
            .values()
            .filter(|f| f.last_agent.starts_with(session_id))
            .cloned()
            .collect()
    }

    pub async fn get_all_agents(&self) -> Vec<AgentState> {
        self.inner.lock().await.agents.values().cloned().collect()
    }

    pub async fn get_active_sessions(&self) -> Vec<AgentState> {
        self.inner
            .lock()
            .await
            .agents
            .values()
            .filter(|a| a.agent_id.is_none() && a.status == AgentStatus::Active)
            .cloned()
            .collect()
    }
}

fn agent_key(session_id: &str, agent_id: Option<&str>) -> String {
    match agent_id {
        Some(id) => format!("{session_id}:{id}"),
        None => session_id.to_owned(),
    }
}

fn upsert_file_access(
    files: &mut HashMap<String, FileAccess>,
    path: &str,
    tool_name: &str,
    agent_key: &str,
    ts: EpochMs,
) {
    if let Some(file) = files.get_mut(path) {
        file.access_count += 1;
        tool_name.clone_into(&mut file.last_tool);
        agent_key.clone_into(&mut file.last_agent);
        file.last_access = ts;
    } else {
        files.insert(
            path.to_owned(),
            FileAccess {
                path: path.to_owned(),
                last_tool: tool_name.to_owned(),
                last_agent: agent_key.to_owned(),
                last_access: ts,
                access_count: 1,
                was_modified: false,
            },
        );
    }
}
