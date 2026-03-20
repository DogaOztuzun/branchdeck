# Branchdeck Architecture

**Generated:** 2026-03-20
**Type:** Desktop application (Tauri v2)
**Pattern:** Multi-layer event-driven with functional core / imperative shell

## 1. System Overview

```
+-------------------------------------------------------------------+
|  SolidJS Frontend (WebView)                                        |
|  Stores (reactive) <-- Tauri events (agent:event, task:updated,   |
|  Components --------> Tauri invoke (IPC commands)                  |
+-----------------------------|-------------------------------------+
                              | Tauri IPC boundary
+-----------------------------|-------------------------------------+
|  Rust Backend (Tauri v2)    v                                      |
|                                                                    |
|  Commands (thin handlers) --> Services (business logic)            |
|                                                                    |
|  EventBus (tokio::broadcast) --> ActivityStore, EventBridge,       |
|                                  KnowledgeIngestion                |
|                                                                    |
|  HookReceiver (TCP:13370) <-- Claude Code command hooks            |
|  RunManager ---stdin/stdout--> Sidecar (Node.js child process)     |
|  KnowledgeMCP (TCP) <-------> knowledge-mcp.js sidecar             |
+-------------------------------------------------------------------+
```

## 2. Architecture Principles

1. **No business logic in commands** - Commands validate args, call a service, return the result
2. **Services are daemon-extractable** - No Tauri types in business logic, dependencies passed as parameters
3. **Functional core / imperative shell** - Pure state transitions return `Vec<RunEffect>`, thin executor applies them
4. **Event-driven communication** - Internal `EventBus` (tokio::broadcast) for agent events, Tauri events for frontend updates
5. **File-based persistence** - `task.md` (YAML frontmatter) and `run.json` as durable state, no database (Phase 1)
6. **Feature-gated optionals** - Knowledge service behind `knowledge` feature, SONA behind `sona` feature

## 3. Backend Architecture

### 3.1 Service Layer (~6,700 LOC across 23 modules)

**Core Services:**
| Service | Type | Responsibility |
|---------|------|----------------|
| `git` | Stateless | Worktree CRUD, branches, status, tracking (git2) |
| `terminal` | Stateful | PTY session lifecycle (portable-pty) |
| `config` | Stateless | Global/per-repo JSON config (SHA256-hashed paths) |
| `github` | Async+cached | PR status, checks, reviews (octocrab, 30s cache) |

**Task/Run System:**
| Service | Type | Responsibility |
|---------|------|----------------|
| `task` | Stateless | task.md YAML parsing, status updates, artifact capture |
| `task_watcher` | Stateful | File watcher (notify crate, 500ms debounce, content-hash dedup) |
| `run_manager` | Stateful | Sidecar lifecycle, stdin/stdout protocol, heartbeat, permissions |
| `run_effects` | Pure | State machine: `(state, input) -> (state, Vec<RunEffect>)` |
| `run_responses` | Thin wrapper | Calls pure functions, executes effects |
| `run_stale` | Stateless | Stale detection (120s), permission timeouts (300s) |
| `run_state` | Stateless | run.json persistence for crash recovery |

**Agent Monitoring:**
| Service | Type | Responsibility |
|---------|------|----------------|
| `event_bus` | Simple | tokio::broadcast channel (256 capacity) |
| `event_bridge` | Async task | Forward EventBus events to Tauri frontend |
| `hook_receiver` | TCP server | Listen :13370, parse hook POSTs, publish to EventBus |
| `hook_config` | Stateless | Generate notify.sh, manage .claude/settings.json hooks |
| `activity_store` | Stateful | Track agent/file activity, GC stale entries (5min TTL) |
| `agent_scanner` | Stateless | Parse .claude/agents/*.md frontmatter |

**Knowledge System (feature-gated):**
| Service | Type | Responsibility |
|---------|------|----------------|
| `knowledge` | Stateful | RVF vector stores, fastembed ONNX embedder |
| `knowledge_ingestion` | Async | Trajectory capture, error pattern mining from EventBus |
| `knowledge_query` | Async | Vector search, re-ranking, metadata filters |
| `knowledge_merge` | Async | Deduplication, pattern extraction |
| `knowledge_mcp` | TCP server | MCP endpoint for Claude Code knowledge tools |

### 3.2 Command Layer (44 IPC commands across 9 modules)

All commands follow: validate args -> call service -> return Result<T, AppError>

| Module | Commands | Service(s) |
|--------|----------|------------|
| git | 10 | git, config |
| terminal | 4 | terminal |
| task | 6 | task, task_watcher |
| run | 7 | run_manager, run_state |
| agent | 5 | activity_store, agent_scanner, hook_config |
| github | 2 | github |
| workspace | 6 | config |
| knowledge | 4 | knowledge (feature-gated) |

### 3.3 Data Models (7 modules)

| Model | Key Types | Serde |
|-------|-----------|-------|
| repo | RepoInfo, WorktreeInfo, WorktreePreview, FileStatus, BranchInfo, TrackingInfo, PrInfo, CheckRunInfo, ReviewInfo | camelCase |
| session | PtySession, PtyEvent, SessionId | camelCase |
| task | TaskStatus (6), TaskType (2), TaskScope (2), TaskFrontmatter, TaskInfo | kebab-case |
| run | RunStatus (7), RunInfo, SidecarRequest (3), SidecarResponse (8), LaunchOptions, PendingPermission | mixed |
| agent | Event (8 variants), AgentState, FileAccess, AgentDefinition, HookPayload | camelCase |
| knowledge | KnowledgeEntry, TrajectoryRecord, QueryResult, Suggestion, PendingEntry | camelCase |

### 3.4 Error Handling

Single `AppError` enum (12 variants) via thiserror: Git, Pty, Config, Io, GitHub, Agent, TaskAlreadyExists, TaskNotFound, TaskParseError, TaskWatchError, RunError, SidecarError, Knowledge. Implements Serialize for Tauri IPC.

### 3.5 RunEffect Pattern (Functional Core)

```rust
// Pure function: no I/O, no AppHandle, deterministic
fn apply_run_complete(run, cost, start, now) -> Vec<RunEffect> {
    vec![
        RunEffect::UpdateTaskStatus(path, TaskStatus::Succeeded),
        RunEffect::CaptureArtifacts(path, "succeeded", start),
        RunEffect::DeleteRunState(path),
        RunEffect::EmitRunStatus(run_info),
        RunEffect::PublishEvent(Event::RunComplete { ... }),
    ]
}

// Thin executor: one match arm per effect variant
fn execute_effects(effects, app_handle, event_bus) {
    for effect in effects {
        match effect {
            RunEffect::UpdateTaskStatus(p, s) => task::update_task_status(&p, s),
            RunEffect::EmitRunStatus(info) => app_handle.emit("run:status_changed", &info),
            // ...
        }
    }
}
```

## 4. Frontend Architecture

### 4.1 Store Pattern (Singleton Factory)

```typescript
// Factory: getRepoStore(), getTaskStore(), etc.
// Singleton via global variable + lazy init
// Uses createStore + produce for mutations
// batch() for multiple Tauri event updates
// listen<T>() for Tauri event subscriptions
```

| Store | State | Event Listeners |
|-------|-------|-----------------|
| repo | repos, worktrees, active selections, statuses, PR/tracking | None (command-driven) |
| task | tasks by worktree, activeRun, runLog, pendingPermissions | task:updated, run:status_changed, run:step, run:permission_request |
| terminal | tabs, activeTab per worktree, output handlers | None (callback-driven) |
| agent | agents by tab, event log | agent:event |
| layout | panel visibility, sidebar view | None (signal-driven) |
| knowledge | stats, loading | None (command-driven) |

### 4.2 Component Architecture

**Shell.tsx** is the root layout using solid-resizable-panels:
- Left panel (18%): RepoSidebar (repo tree, worktrees, PR badges)
- Center panel (64%): TerminalArea (multi-tab xterm.js + agent activity)
- Right panel (18%): ChangesSidebar | TeamSidebar | TaskDashboard (toggled)

### 4.3 IPC Pattern

```typescript
// src/lib/commands/git.ts
export async function listWorktrees(repoPath: string): Promise<WorktreeInfo[]> {
  try {
    return await invoke<WorktreeInfo[]>('list_worktrees_cmd', { repoPath });
  } catch (e) {
    logError(`listWorktrees failed: ${e}`);
    throw e;
  }
}
```

## 5. Sidecar Architecture

### 5.1 Agent Bridge (`sidecar/agent-bridge.js`)

Stdin/stdout JSON protocol between Rust RunManager and Claude Agent SDK:

**Inbound (stdin):** `launch_run`, `resume_run`, `cancel_run`, `permission_response`
**Outbound (stdout):** `session_started`, `run_complete`, `run_error`, `permission_request`, `tool_call`, `assistant_text`, `run_step`, `heartbeat`

- 30s heartbeat during active runs
- Forwards hook events to HookReceiver via HTTP POST
- Manages pending permissions as Map<toolUseId, resolveFunction>

### 5.2 Knowledge MCP (`sidecar/knowledge-mcp.js`)

JSON-RPC over stdio, exposes 3 tools to Claude Code:
- `query_knowledge` - Semantic search in vector store
- `remember_this` - Ingest explicit knowledge
- `suggest_next` - SONA-based next-step suggestions

Proxies to Rust knowledge service via HTTP (port from `BRANCHDECK_KNOWLEDGE_PORT`).

## 6. Event Architecture

### 6.1 Internal Events (EventBus)

```
Claude Code hooks (POST :13370)
    -> HookReceiver (parse, construct Event)
    -> EventBus (tokio::broadcast, 256 cap)
        -> ActivityStore (agent/file tracking)
        -> EventBridge (-> Tauri "agent:event")
        -> KnowledgeIngestion (trajectory capture)
```

8 event variants: SessionStart, ToolStart, ToolEnd, SubagentStart, SubagentStop, SessionStop, Notification, RunComplete

### 6.2 Tauri Events (Backend -> Frontend)

| Event | Emitter | Listener |
|-------|---------|----------|
| `agent:event` | EventBridge | agent store |
| `task:updated` | TaskWatcher, recovery | task store |
| `run:status_changed` | RunManager effects | task store |
| `run:step` | RunManager | task store |
| `run:permission_request` | RunManager effects | task store |
| `pty:{session_id}` | Terminal reader task | terminal store (per-session Channel) |

## 7. Startup Sequence

1. Create EventBus + ActivityStore
2. Initialize plugins (opener, dialog, global-shortcut, log)
3. Manage TerminalService, ActivityStore, EventBus, AgentMonitorConfig, TaskWatcher
4. **Setup:**
   a. Resolve sidecar path (agent-bridge.js)
   b. Create RunManager state
   c. **Recover stale runs** - scan all worktrees for orphaned run.json, mark active ones as failed
   d. **Setup agent monitoring** - start ActivityStore subscriber, start HookReceiver, start EventBridge
   e. **Start stale checker** - 30s interval, checks for stalled runs + permission timeouts
   f. **Initialize knowledge service** (feature-gated) - open RVF stores, start trajectory subscriber, start MCP endpoint
5. Register 44 invoke handlers
6. Register window close handler (shutdown RunManager, cleanup MCP, close knowledge stores, close terminals)

## 8. Deployment Architecture

- **Build:** `bunx tauri build` produces AppImage (Linux), deb, rpm
- **Bundled resources:** `sidecar/agent-bridge.js`, `sidecar/knowledge-mcp.js`
- **Config dir:** `~/.config/branchdeck/` (global config, repo configs, hooks)
- **Release:** GitHub Actions manual dispatch -> version bump -> tag -> build -> GitHub release
- **Auto-update:** Not yet implemented

## 9. Testing Strategy

| Layer | Tool | Approach |
|-------|------|----------|
| Rust pure functions | cargo test | Test `apply_*` functions, assert on effects (order-independent) |
| Rust integration | cargo test + tempfile/git2 | Temp repos, real git operations |
| Frontend utils | vitest | parseArtifactSummary, statusColor, shortPath |
| Commands/RunManager | Not tested | Require full Tauri runtime |
| Terminal/PTY | Not tested | Needs real terminal |
| Knowledge | Not tested | Needs ONNX model download |
