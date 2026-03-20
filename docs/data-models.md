# Data Models

**Generated:** 2026-03-20

## Overview

Branchdeck uses 7 model modules in `src-tauri/src/models/` defining domain types that are serialized for IPC. No database - persistence is via JSON config files and YAML-frontmatter markdown (task.md). Knowledge uses RVF binary vector stores.

## 1. Repository & Git Domain

### RepoInfo
```rust
{ name: String, path: PathBuf, current_branch: String }
```
Returned by `validate_repo()`. Represents a git repository added to the workspace.

### WorktreeInfo
```rust
{ name: String, path: PathBuf, branch: String, is_main: bool }
```
A git worktree (main or linked). `is_main` distinguishes the bare repo worktree from created worktrees.

### WorktreePreview
```rust
{ sanitized_name, branch_name, worktree_path, base_branch, branch_exists, path_exists, worktree_exists }
```
Preview state for worktree creation modal - shows conflicts before creation.

### FileStatus
```rust
{ path: String, status: String }  // status: "new" | "modified" | "deleted" | "renamed" | "conflicted"
```

### BranchInfo
```rust
{ name: String, is_remote: bool, is_head: bool, has_worktree: bool }
```

### TrackingInfo
```rust
{ ahead: usize, behind: usize, upstream_name: String }
```
Branch ahead/behind counts relative to upstream.

### PrInfo
```rust
{ number, title, state, is_draft, url, checks: Vec<CheckRunInfo>, reviews: Vec<ReviewInfo>,
  additions, deletions, review_decision }
```
GitHub PR status aggregated from octocrab. Cached for 30s.

### CheckRunInfo
```rust
{ name, conclusion, status, details_url }
```
Individual CI check run. Deduplicated by name (latest wins).

### ReviewInfo
```rust
{ user, state, submitted_at }
```
PR review. Latest per user determines review decision.

## 2. Task Domain

### TaskStatus (enum)
`Created | Running | Blocked | Succeeded | Failed | Cancelled`
Serialized as kebab-case.

### TaskType (enum)
`IssueFix | PrShepherd`
Built-in task types. Custom types planned for Phase 4.

### TaskScope (enum)
`Worktree | Workspace`
Worktree = single branch/repo. Workspace = cross-repo (Phase 3).

### TaskFrontmatter
```rust
{ task_type, scope, status, repo, branch, pr: Option<u64>, created, run_count }
```
YAML frontmatter in `.branchdeck/task.md`. The shared protocol between human, agent, and Branchdeck core.

### TaskInfo
```rust
{ frontmatter: TaskFrontmatter, body: String, path: String }
```
Parsed task file with frontmatter and markdown body.

## 3. Run Domain

### RunStatus (enum)
`Created | Starting | Running | Blocked | Succeeded | Failed | Cancelled`
Serialized as kebab-case.

### RunInfo
```rust
{ session_id, task_path, status, started_at, cost_usd, last_heartbeat, elapsed_secs, tab_id }
```
Current run state. Persisted to `.branchdeck/run.json` for crash recovery.

### SidecarRequest (tagged enum)
```rust
LaunchRun { task_path, worktree, options, hook_port, tab_id }
ResumeRun { task_path, worktree, session_id, options, hook_port, tab_id }
CancelRun { session_id }
```
Sent to sidecar via stdin JSON.

### SidecarResponse (tagged enum, 8 variants)
```rust
Heartbeat, SessionStarted, RunStep, AssistantText, ToolCall, RunComplete, RunError, PermissionRequest
```
Received from sidecar via stdout JSON line protocol.

### LaunchOptions
```rust
{ max_turns: Option<u32>, max_budget_usd: Option<f64> }
```

### PendingPermission
```rust
{ tool, command, tool_use_id, requested_at: u64 }
```
Tracked in RunManager for timeout handling (300s auto-deny).

## 4. Agent/Event Domain

### Event (tagged enum, 8 variants)
```rust
SessionStart { session_id, tab_id, model, ts }
ToolStart { session_id, agent_id, tab_id, tool_name, tool_use_id, file_path, ts }
ToolEnd { session_id, agent_id, tab_id, tool_name, tool_use_id, file_path, ts }
SubagentStart { session_id, agent_id, agent_type, tab_id, ts }
SubagentStop { session_id, agent_id, agent_type, tab_id, ts }
SessionStop { session_id, tab_id, ts }
Notification { session_id, tab_id, title, message, ts }
RunComplete { session_id, tab_id, status, cost_usd, elapsed_secs, ts }
```
Internal events published via EventBus. Routed by `tab_id` (injected as env var into PTY sessions).

### AgentState
```rust
{ session_id, agent_id, agent_type, tab_id, status: AgentStatus, current_tool, current_file,
  started_at, last_activity }
```
In-memory agent tracking in ActivityStore. GC'd after 5min inactivity.

### FileAccess
```rust
{ path, last_tool, last_agent, last_access, access_count, was_modified }
```
Tracks which files agents are reading/writing for the FileGrid heatmap.

### AgentDefinition
```rust
{ name, description, model, tools: Vec<String>, permission_mode, file_path }
```
Parsed from `.claude/agents/*.md` frontmatter.

### HookPayload
```rust
{ session_id, hook_event_name, tool_name, tool_input, tool_use_id, agent_id, agent_type,
  message, title, model }
```
Raw JSON from Claude Code command hooks. Converted to typed Event in HookReceiver.

## 5. Knowledge Domain

### KnowledgeType (enum)
`Trajectory | Commit | Explicit | ErrorResolution | Pattern`

### KnowledgeSource (enum)
`EventBus | Mcp | User | GitHook`

### KnowledgeEntry
```rust
{ id, content, entry_type, source, repo_hash, worktree_id, metadata: KnowledgeMetadata, created_at }
```
Core knowledge unit stored in RVF vector store. Content is embedded via fastembed (ONNX).

### KnowledgeMetadata
```rust
{ session_id, tool_names, file_paths, run_status, cost_usd, quality_score: f32 }
```

### TrajectoryRecord
```rust
{ session_id, tab_id, steps: Vec<TrajectoryStep>, quality_score, started_at, ended_at }
```
Records the sequence of tool calls in a session for learning.

### QueryResult
```rust
{ id, content, entry_type, distance: f32, metadata }
```
Vector similarity search result.

### Suggestion (SONA feature)
```rust
{ id, content, distance: f32, avg_quality: f32 }
```

## 6. Session/PTY Domain

### PtySession
```rust
{ writer: Box<dyn Write + Send>, master: Box<dyn MasterPty + Send>, child: Box<dyn Child + Send + Sync> }
```
Not serializable - internal to TerminalService.

### PtyEvent (tagged enum)
```rust
Output { bytes: Vec<u8> }
Exit { code: Option<i32> }
```
Streamed to frontend via Tauri Channel (per-session).

## 7. Configuration

### GlobalConfig (services/config.rs)
```json
{ "window": { "width": 1200, "height": 800 }, "defaultShell": "/bin/bash",
  "repos": ["/path/to/repo1", ...], "lastActiveRepo": "/path/to/repo" }
```
Stored at `~/.config/branchdeck/config.json`.

### RepoConfig (services/config.rs)
```json
{ "path": "/path/to/repo", "lastWorktree": "main", "sidebarCollapsed": false,
  "presets": [{ "name": "Dev", "command": "bun run dev", "tabType": "shell" }] }
```
Stored at `~/.config/branchdeck/repo-{sha256hash}.json`.

## 8. Persistence Model

| Data | Location | Format | Lifecycle |
|------|----------|--------|-----------|
| Global config | `~/.config/branchdeck/config.json` | JSON | Persistent |
| Per-repo config | `~/.config/branchdeck/repo-{hash}.json` | JSON | Persistent per repo |
| Task | `{worktree}/.branchdeck/task.md` | YAML frontmatter + Markdown | Per-task lifecycle |
| Run state | `{worktree}/.branchdeck/run.json` | JSON | Active run only, deleted on completion |
| Knowledge | `~/.config/branchdeck/knowledge/` | RVF binary + JSONL index | Persistent, GC-managed |
| Hook script | `~/.config/branchdeck/hooks/notify.sh` | Bash | Generated on startup |
| Hook config | `~/.claude/settings.json` | JSON (merged) | Installed on startup, cleaned on shutdown |
