# Agent Monitoring & Orchestration

> Architecture spec for real-time Claude Code agent monitoring inside Branchdeck. Phase 1 (foundation) is implementation-ready. Later phases are directional.

**Status:** Draft v3 — adversarial review fixes applied
**Date:** 2026-03-17
**References:** [Superset](https://github.com/nichochar/superset) (command hooks pattern), [Arbor](https://github.com/cyanff/arbor) (broadcast + WebSocket pattern)

---

## 1. Problem

Claude Code agents run in terminal sessions — opaque black boxes. You cannot see:
- Which files an agent is reading/writing right now
- Whether a subagent was spawned and what it is doing
- How agents in a team coordinate
- Cost/token burn rate across sessions

Branchdeck already manages terminal sessions and worktrees. Adding agent observability makes it the control plane for AI-assisted development.

---

## 2. Goals

| Goal | Description |
|------|-------------|
| **Monitor** | Real-time visibility into all agent activity (tool calls, file access, subagent lifecycle) |
| **Visualize** | File dot grid showing which files each agent is touching, with hover tooltips |
| **Orchestrate** | Launch, resume, and manage predefined agents from a team sidebar |
| **Track** | Token usage, cost, and session duration per agent |

### Non-goals (v1)
- Custom agent editor/IDE (use `.claude/agents/*.md` directly)
- Cross-machine agent monitoring (local only)
- Mid-turn prompt injection (SDK limitation — between turns only)

---

## 3. Architecture Overview

### 3.1 Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hook transport | **Command hooks** (not HTTP hooks) | Claude Code fires a shell command; the command curls our localhost listener. Cross-platform. |
| HTTP listener | **Raw `tokio::net::TcpListener`** | Manual HTTP POST parsing with Content-Length. No Axum, no framework. We already have tokio. |
| Internal pub/sub | **`tokio::sync::broadcast`** | Typed events. Multiple subscribers. No new deps. |
| Timestamps | **Epoch milliseconds (u64)** | No chrono dependency. `SystemTime::now()` → epoch ms. |
| New dependencies | **ZERO** | tokio (have it), serde/serde_json (have it). Nothing else. |
| Crate structure | **Single crate** | Code organized into modules that map to future crate boundaries. |
| Mutex type | **`tokio::sync::Mutex`** | Async Mutex for ActivityStore — std::sync::Mutex blocks the tokio runtime. |

### 3.2 System Diagram

```
┌──────────────────────────────────────────────────────────────────┐
│  SolidJS Frontend                                                 │
│  ┌────────────┐  ┌────────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ File Grid  │  │ Terminal   │  │ Agent    │  │ Team         │  │
│  │ (dot map)  │  │ Area       │  │ Activity │  │ Sidebar      │  │
│  └─────▲──────┘  └────────────┘  └────▲─────┘  └──────┬───────┘  │
│        │ Tauri events                 │                │          │
├────────┼──────────────────────────────┼────────────────┼──────────┤
│  Rust Backend                         │                │          │
│                                       │                │          │
│  ┌────────────────────────────────────┴────────────────┘          │
│  │                                                                │
│  │    ┌──────────────────────────────────────────┐                │
│  │    │         tokio::sync::broadcast           │                │
│  │    │              (EventBus)                   │                │
│  │    └──────┬──────────┬────────────┬───────────┘                │
│  │           │          │            │                             │
│  │    ┌──────▼───┐ ┌────▼─────┐ ┌───▼──────────┐                 │
│  │    │Activity  │ │Tauri     │ │Future:       │                 │
│  │    │Store     │ │Event     │ │Webhook fwd,  │                 │
│  │    │(state)   │ │Bridge    │ │Orchestrator  │                 │
│  │    └──────────┘ └──────────┘ └──────────────┘                 │
│  │                                                                │
│  │    ┌──────────────┐  ┌──────────────────┐                      │
│  │    │Hook Receiver │  │Hook Config       │                      │
│  │    │TcpListener   │  │Manager           │                      │
│  │    │:13370        │  │(notify.sh,       │                      │
│  │    │parses POST,  │  │ settings.json)   │                      │
│  │    │publishes to  │  │                  │                      │
│  │    │EventBus      │  │                  │                      │
│  │    └──────────────┘  └──────────────────┘                      │
│  │                                                                │
│  │    ┌──────────────┐                                            │
│  │    │Agent Scanner │  (.claude/agents/*.md)                     │
│  │    └──────────────┘                                            │
│  │                                                                │
│  └────────────────────────────────────────────────────────────────┘
│                                                                    │
│  Existing: TerminalService, GitService, ConfigService              │
└────────────────────────────────────────────────────────────────────┘
```

### 3.3 Event Flow (end-to-end)

Routing uses `BRANCHDECK_TAB_ID` as the primary key (injected as an env var into each PTY session). The `cwd` field from hook payloads is informational only — worktrees have different cwds than the main repo, so cwd-based routing would be unreliable.

```
Claude Code starts / executes a tool
  │
  ▼
SessionStart / PreToolUse hook fires
  │
  ▼
Shell command: notify.sh (generated by Branchdeck)
  │  reads $BRANCHDECK_PORT, $BRANCHDECK_TAB_ID, $BRANCHDECK_SESSION_ID
  │
  ▼
curl POST http://127.0.0.1:13370/hook
  │  body: JSON from stdin + env var routing context
  │
  ▼
Hook Receiver (TcpListener)
  │  reads Content-Length, parses JSON (max 64KB)
  │  constructs typed Event
  │
  ▼
EventBus (broadcast channel)
  │
  ├──▶ ActivityStore subscriber (fast, no blocking I/O)
  │      updates in-memory agent + file state
  │
  ├──▶ Tauri Event Bridge subscriber
  │      emits "agent:event" to frontend
  │
  └──▶ (future) Webhook forwarder, Orchestrator
```

---

## 4. Data Sources

### 4.1 Command Hooks (primary)

Claude Code supports `command` hooks in `.claude/settings.json`. Each hook event pipes its JSON payload to stdin of the configured command.

**Hook configuration (auto-generated by Branchdeck):**

Note: paths are absolute, resolved at runtime by `ensure_notify_script()`. Tilde (`~`) is not expanded in JSON.

```json
{
  "hooks": {
    "SessionStart": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "PreToolUse": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "PostToolUse": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "SubagentStart": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "SubagentStop": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "Stop": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }],
    "Notification": [{ "type": "command", "command": "/home/user/.config/branchdeck/hooks/notify.sh" }]
  }
}
```

### 4.2 Hook Script Template

Generated at the absolute path returned by `ensure_notify_script()` (e.g., `/home/user/.config/branchdeck/hooks/notify.sh`) on app startup. **No `jq` dependency** — uses pure bash string manipulation:

```bash
#!/usr/bin/env bash
# Auto-generated by Branchdeck. Do not edit.
PORT="${BRANCHDECK_PORT:-13370}"
TAB_ID="${BRANCHDECK_TAB_ID:-}"
SESSION_ID="${BRANCHDECK_SESSION_ID:-}"
PAYLOAD=$(cat)
# Inject routing context by replacing the closing } with additional fields
BODY="${PAYLOAD%\}},\"branchdeck_tab_id\":\"${TAB_ID}\",\"branchdeck_session_id\":\"${SESSION_ID}\"}"
curl -s -X POST "http://127.0.0.1:${PORT}/hook" \
  -H "Content-Type: application/json" \
  -d "$BODY" --max-time 2 > /dev/null 2>&1 || true
```

### 4.3 Hook Event Payloads

Common fields from Claude Code:

| Field | Description |
|-------|-------------|
| `session_id` | UUID — identifies the Claude session |
| `transcript_path` | Absolute path to session .jsonl |
| `cwd` | Working directory (informational, not used for routing) |
| `hook_event_name` | Event type string |

Event-specific fields:

| Event | Extra Fields |
|-------|-------------|
| `SessionStart` | `model` |
| `PreToolUse` | `tool_name`, `tool_input`, `tool_use_id` |
| `PostToolUse` | `tool_name`, `tool_input`, `tool_response`, `tool_use_id` |
| `SubagentStart` | `agent_id`, `agent_type` |
| `SubagentStop` | `agent_id`, `agent_type`, `agent_transcript_path` |
| `Stop` | `last_assistant_message` |
| `Notification` | `message`, `title`, `notification_type` |

### 4.4 File Path Extraction

| Tool | Path field(s) |
|------|--------------|
| `Read` / `Write` / `Edit` | `tool_input.file_path` |
| `Glob` | `tool_input.path` |
| `Grep` | `tool_input.path` |
| `Bash` | Best-effort parse of `tool_input.command` |

### 4.5 Agent Definition Files

Location: `.claude/agents/*.md` — simple key-value frontmatter between `---` delimiters with `name`, `description`, `model`, `tools`, `permission_mode`. See section 6.5 for parsing details.

---

## 5. Rust Types

### 5.1 Event and State Types (`models/agent.rs`)

```rust
pub type EpochMs = u64;

pub fn now_ms() -> EpochMs {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Event {
    SessionStart { session_id: String, tab_id: String, model: Option<String>, ts: EpochMs },
    ToolStart { session_id: String, agent_id: Option<String>, tab_id: String, tool_name: String, tool_use_id: String, file_path: Option<String>, ts: EpochMs },
    ToolEnd { session_id: String, agent_id: Option<String>, tab_id: String, tool_name: String, tool_use_id: String, file_path: Option<String>, ts: EpochMs },
    SubagentStart { session_id: String, agent_id: String, agent_type: String, tab_id: String, ts: EpochMs },
    SubagentStop { session_id: String, agent_id: String, agent_type: String, tab_id: String, ts: EpochMs },
    SessionStop { session_id: String, tab_id: String, ts: EpochMs },
    Notification { session_id: String, tab_id: String, title: Option<String>, message: String, ts: EpochMs },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus { Active, Idle, Stopped }

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
    #[serde(default)] pub tool_name: Option<String>,
    #[serde(default)] pub tool_input: Option<serde_json::Value>,
    #[serde(default)] pub tool_use_id: Option<String>,
    #[serde(default)] pub agent_id: Option<String>,
    #[serde(default)] pub agent_type: Option<String>,
    #[serde(default)] pub message: Option<String>,
    #[serde(default)] pub title: Option<String>,
    #[serde(default)] pub model: Option<String>,
    #[serde(default)] pub branchdeck_tab_id: Option<String>,
    #[serde(default)] pub branchdeck_session_id: Option<String>,
}
```

### 5.2 EventBus (`services/event_bus.rs`)

```rust
use tokio::sync::broadcast;

const BUS_CAPACITY: usize = 256;

pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BUS_CAPACITY);
        Self { tx }
    }
    pub fn publish(&self, event: Event) -> usize { self.tx.send(event).unwrap_or(0) }
    pub fn subscribe(&self) -> broadcast::Receiver<Event> { self.tx.subscribe() }
}
```

---

## 6. Services

### 6.1 Hook Receiver (`services/hook_receiver.rs`)

Raw `tokio::net::TcpListener` on `:13370`. Reads the `Content-Length` header and allocates a buffer accordingly, with a maximum payload cap of 64KB. Payloads exceeding 64KB (e.g., large `PostToolUse` `tool_response` fields) are dropped with a warning log. Deserializes JSON, converts `HookPayload` → `Event`, publishes to EventBus. ~100 lines, no framework.

On startup, port availability is checked before spawning the listener task. If binding fails, a `tokio::sync::oneshot` channel reports the error back to `setup()`, which logs the failure and disables agent monitoring for the session (non-fatal to app startup).

### 6.2 Hook Config Manager (`services/hook_config.rs`)

- `ensure_notify_script()` — generates notify.sh in the Branchdeck config directory, makes executable, returns the **absolute resolved path** (using `dirs::config_dir()`, no tilde)
- `install_hooks(repo_path, script_path)` — merges hook entries into `.claude/settings.json`, preserves user hooks, idempotent. Uses **atomic writes** (write to temp file, rename) to prevent corruption
- `remove_hooks(repo_path, script_path)` — removes Branchdeck entries from settings, called on app close. Also uses atomic writes

### 6.3 ActivityStore (`services/activity_store.rs`)

In-memory state behind `Arc<tokio::sync::Mutex<>>`. Uses tokio's async Mutex (not `std::sync::Mutex`) because the store is accessed from async subscriber tasks — a std Mutex would block the tokio runtime.

Subscribes to EventBus via a spawned task. The subscriber must be fast: no blocking I/O, minimal computation. GC runs on a separate `tokio::time::interval` timer (e.g., every 60s), not inline with event processing. If the broadcast channel lags and events are dropped, the store logs a warning.

Methods: `handle_event()`, `get_all_agents()`, `get_all_files()`, `gc(ttl)`.

### 6.4 Tauri Event Bridge (`services/event_bridge.rs`)

Spawned task that subscribes to EventBus and emits `"agent:event"` to frontend via `app_handle.emit()`. Single event channel, frontend switches on `event.kind`.

### 6.5 Agent Scanner (`services/agent_scanner.rs`)

Reads `.claude/agents/*.md`, extracts frontmatter between `---` delimiters, and parses it as line-by-line `key: value` pairs (not a full YAML parser). This handles the subset we need — simple string values and bracket-delimited string arrays (e.g., `tools: ["Read", "Write", "Edit"]`). Complex YAML features (nested objects, multi-line strings, anchors) are not supported. No YAML parser dependency needed.

### 6.6 Env Var Injection

When spawning Claude PTY sessions, inject into the env map:
- `BRANCHDECK_PORT` — hook receiver port
- `BRANCHDECK_TAB_ID` — which terminal tab (primary routing key)
- `BRANCHDECK_SESSION_ID` — Branchdeck's session ID

No changes to `TerminalService` — it already accepts `env: &HashMap<String, String>`.

---

## 7. Implementation Phases

### Phase 1: Foundation (implement now)
- [ ] Event, AgentState, FileAccess, HookPayload types (including SessionStart variant)
- [ ] EventBus (broadcast channel)
- [ ] Hook receiver (TCP listener with Content-Length parsing, 64KB cap, port bind check via oneshot)
- [ ] Hook config manager (notify.sh without jq + atomic settings.json merge/clean)
- [ ] ActivityStore (in-memory, tokio::sync::Mutex, event subscriber, separate GC timer)
- [ ] Tauri event bridge (EventBus → frontend)
- [ ] Agent definition scanner (line-by-line frontmatter parsing, no YAML dep)
- [ ] IPC commands (get_agents, get_file_activity, list_definitions, install/remove hooks)
- [ ] Startup wiring in lib.rs (EventBus, port check, spawn receiver/bridge/store)
- [ ] Cleanup on window close: remove hooks BEFORE killing terminal sessions, atomic writes
- [ ] Frontend: env var injection for Claude tabs (BRANCHDECK_PORT, TAB_ID, SESSION_ID)
- [ ] Frontend: call install_agent_hooks on repo select, remove on deselect
- [ ] Frontend: TypeScript types + IPC wrappers
- **Dependencies: ZERO new**

### Phase 2: Monitoring UI
- [ ] `src/lib/stores/agent.ts` — SolidJS store + `agent:event` listener
- [ ] `AgentBadge.tsx` — status dot + name + current tool
- [ ] `AgentActivity.tsx` — scrolling event feed
- [ ] Integration into Shell.tsx layout

### Phase 3: File Dot Grid
- [ ] `get_repo_files` command (git index listing)
- [ ] `FileGrid.tsx` — dot visualization (idle/read/modified/active)
- [ ] `FileGridTooltip.tsx` — hover details
- [ ] Center pane toggle: Terminal / Split / Grid

### Phase 4: Team Sidebar
- [ ] `TeamSidebar.tsx` — agent definitions + launch button
- [ ] Agent launch via `claude --agent <name>` in PTY
- [ ] Agent grouping by task/thread

---

## 8. Future Vision

Not implemented in Phase 1-4. The EventBus is designed to support these without architectural changes.

- **Workflow state machines** — YAML-defined multi-step agent orchestration via an orchestrator subscriber
- **Multi-crate workspace** — extract to `branchdeck-core`, `branchdeck-server`, `branchdeck-hooks`, `branchdeck-agents`
- **Web app / CLI** — same event system, different frontends
- **Remote server** — hook receiver on a remote machine
- **Cross-agent communication** — agents publish/subscribe to EventBus topics
- **Webhook forwarder** — EventBus subscriber that POSTs to Slack/Discord
- **Persistence** — optional SQLite subscriber for event replay

---

## 9. Open Questions

1. ~~**Port collision**~~ **Resolved:** Port availability is checked synchronously at startup. Bind failure disables monitoring with a user-visible warning (non-fatal).
2. **Hook cleanup on crash:** On next launch, check if notify.sh exists and port is reachable. Clean stale hooks if not.
3. ~~**Multi-repo routing**~~ **Resolved:** One hook receiver. Route by `BRANCHDECK_TAB_ID` (injected env var), not `cwd`. One ActivityStore with per-tab views.
4. ~~**jq dependency**~~ **Resolved:** notify.sh uses pure bash string manipulation. No jq required.
5. **Large repos:** File dot grid with 10k+ files needs virtualized rendering. Defer to Phase 3.
