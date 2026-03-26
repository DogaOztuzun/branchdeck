use crate::error::AppError;
use crate::models::agent::{now_ms, AgentState, AgentStatus, EpochMs, Event, FileAccess};
use crate::services::event_bus::EventBus;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Maximum events retained in the JSONL persistence file.
/// On load, older events beyond this limit are discarded and the file is compacted.
const MAX_PERSISTED_EVENTS: usize = 10_000;

struct StoreInner {
    agents: HashMap<String, AgentState>,
    files: HashMap<String, FileAccess>,
    /// Raw events kept for overnight summary queries.
    events: Vec<Event>,
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
    /// Path to the JSONL persistence file. `None` means in-memory only.
    persistence_path: Option<PathBuf>,
}

impl Default for ActivityStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityStore {
    /// Create an in-memory-only activity store (no persistence across restarts).
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StoreInner {
                agents: HashMap::new(),
                files: HashMap::new(),
                events: Vec::new(),
            })),
            persistence_path: None,
        }
    }

    /// Create an activity store backed by a JSONL file in `data_dir`.
    ///
    /// Events are appended to `{data_dir}/activity.jsonl` and loaded on construction.
    /// If loading fails, the store starts empty and logs the error.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Io` if the data directory cannot be created.
    pub fn new_with_persistence(data_dir: &std::path::Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            error!("Failed to create activity data dir {}: {e}", data_dir.display());
            e
        })?;

        let persistence_path = data_dir.join("activity.jsonl");

        let mut store = Self {
            inner: Arc::new(Mutex::new(StoreInner {
                agents: HashMap::new(),
                files: HashMap::new(),
                events: Vec::new(),
            })),
            persistence_path: Some(persistence_path),
        };

        if let Err(e) = store.load_persisted_events() {
            error!("Failed to load persisted activity events: {e}");
        }

        Ok(store)
    }

    /// Load events from the JSONL file, replay them into the in-memory store,
    /// and compact the file if it exceeds `MAX_PERSISTED_EVENTS`.
    fn load_persisted_events(&mut self) -> Result<(), AppError> {
        let path = match &self.persistence_path {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("No persisted activity file at {}, starting fresh", path.display());
                return Ok(());
            }
            Err(e) => return Err(AppError::Io(e)),
        };

        let reader = BufReader::new(file);
        let mut events = Vec::new();
        let mut parse_errors = 0u64;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<Event>(&line) {
                Ok(event) => events.push(event),
                Err(_) => parse_errors += 1,
            }
        }

        if parse_errors > 0 {
            warn!("Skipped {parse_errors} unparseable lines from {}", path.display());
        }

        // Truncate to last MAX_PERSISTED_EVENTS
        let needs_compact = events.len() > MAX_PERSISTED_EVENTS;
        if needs_compact {
            let drain_count = events.len() - MAX_PERSISTED_EVENTS;
            events.drain(..drain_count);
        }

        let event_count = events.len();
        self.replay_events_sync(&events)?;

        info!("Loaded {event_count} persisted activity events from {}", path.display());

        // Compact the file if we truncated
        if needs_compact {
            if let Err(e) = self.compact_persistence_file(&events) {
                error!("Failed to compact activity file: {e}");
            }
        }

        Ok(())
    }

    /// Replay events into the in-memory store (blocking, used during init).
    fn replay_events_sync(&self, events: &[Event]) -> Result<(), AppError> {
        // We can't use async lock during init, so use try_lock
        let mut inner = self.inner.try_lock().map_err(|e| {
            error!("Failed to acquire lock for replay: {e}");
            AppError::Agent(format!("Lock contention during event replay: {e}"))
        })?;

        inner.events = events.to_vec();

        for event in events {
            match event {
                Event::SessionStart {
                    session_id,
                    tab_id,
                    ts,
                    ..
                } => inner.on_session_start(session_id.clone(), tab_id.clone(), *ts),
                Event::ToolStart {
                    session_id,
                    agent_id,
                    tool_name,
                    file_path,
                    ts,
                    ..
                } => inner.on_tool_start(
                    session_id,
                    agent_id.as_deref(),
                    tool_name,
                    file_path.as_deref(),
                    *ts,
                ),
                Event::ToolEnd {
                    session_id,
                    agent_id,
                    tool_name,
                    file_path,
                    ts,
                    ..
                } => inner.on_tool_end(
                    session_id,
                    agent_id.as_deref(),
                    tool_name,
                    file_path.as_deref(),
                    *ts,
                ),
                Event::SubagentStart {
                    session_id,
                    agent_id,
                    agent_type,
                    tab_id,
                    ts,
                } => inner.on_subagent_start(
                    session_id.clone(),
                    agent_id.clone(),
                    agent_type.clone(),
                    tab_id.clone(),
                    *ts,
                ),
                Event::SubagentStop {
                    session_id,
                    agent_id,
                    ts,
                    ..
                } => inner.on_subagent_stop(session_id, agent_id, *ts),
                Event::SessionStop { session_id, ts, .. } => {
                    inner.on_session_stop(session_id, *ts);
                }
                Event::Notification { session_id, ts, .. } => {
                    if let Some(agent) = inner.agents.get_mut(session_id.as_str()) {
                        agent.last_activity = *ts;
                    }
                }
                Event::RunComplete { .. }
                | Event::PrStatusChanged { .. }
                | Event::RetryDue { .. }
                | Event::IssueDetected { .. }
                | Event::PrMerged { .. } => {}
            }
        }
        Ok(())
    }

    /// Rewrite the JSONL file with only the given events (atomic via `write_atomic`).
    fn compact_persistence_file(&self, events: &[Event]) -> Result<(), AppError> {
        let Some(path) = &self.persistence_path else {
            return Ok(());
        };

        let mut content = String::new();
        for event in events {
            if let Ok(line) = serde_json::to_string(event) {
                content.push_str(&line);
                content.push('\n');
            }
        }

        crate::util::write_atomic(path, content.as_bytes())?;
        info!("Compacted activity file to {} events at {}", events.len(), path.display());
        Ok(())
    }

    /// Append a single event as a JSON line to the persistence file.
    ///
    /// Append-only JSONL persistence — intentional exception to `write_atomic` rule.
    /// Rewriting the entire file per event (`write_atomic`) is O(n) and untenable at 10k events.
    /// Compaction uses `write_atomic`; hot-path appends are append-only by design.
    fn persist_event(&self, event: &Event) {
        let Some(path) = &self.persistence_path else {
            return;
        };

        let line = match serde_json::to_string(event) {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to serialize event for persistence: {e}");
                return;
            }
        };

        let result = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| writeln!(f, "{line}"));

        if let Err(e) = result {
            error!("Failed to persist activity event to {}: {e}", path.display());
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
        // RetryDue events have no timestamp and aren't useful on replay — skip persistence
        if matches!(&event, Event::RetryDue { .. }) {
            return;
        }

        // Persist before processing so the event survives a crash
        self.persist_event(&event);

        let mut inner = self.inner.lock().await;
        inner.events.push(event.clone());

        // Trim in-memory events list to avoid unbounded growth
        if inner.events.len() > MAX_PERSISTED_EVENTS {
            let drain_count = inner.events.len() - MAX_PERSISTED_EVENTS;
            inner.events.drain(..drain_count);
        }

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

    /// Get all events recorded since a given timestamp.
    /// Used by the overnight summary to query activity from the last session.
    pub async fn get_events_since(&self, since_ms: EpochMs) -> Vec<Event> {
        self.inner
            .lock()
            .await
            .events
            .iter()
            .filter(|e| e.timestamp() >= since_ms)
            .cloned()
            .collect()
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
            .filter(|f| {
                f.last_agent == session_id
                    || f.last_agent.starts_with(&format!("{session_id}:"))
            })
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
