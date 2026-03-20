# Source Tree Analysis

**Generated:** 2026-03-20

## Project Structure

```
branchdeck/
|-- .github/workflows/              # CI/CD pipelines (5 workflows)
|   |-- ci.yml                      # PR checks: Biome + Clippy + fmt + build + tests
|   |-- claude-ci-fix.yml           # Auto-fix CI failures on feature branches
|   |-- claude-code-review.yml      # Auto-review PRs with inline comments
|   |-- claude.yml                  # Respond to @claude mentions on PRs/issues
|   +-- release.yml                 # Manual dispatch: version bump -> tag -> build -> release
|
|-- sidecar/                        # Node.js sidecar processes (bundled as Tauri resources)
|   |-- package.json                # deps: @anthropic-ai/claude-agent-sdk
|   |-- agent-bridge.js             # Claude SDK <-> Branchdeck stdin/stdout protocol bridge
|   +-- knowledge-mcp.js            # MCP server for knowledge tools (query, remember, suggest)
|
|-- src/                            # SolidJS frontend (TypeScript)
|   |-- App.tsx                     # Root component, keyboard shortcuts
|   |-- index.tsx                   # Bootstrap, renders into #root
|   |-- index.css                   # Tailwind v4 + Tokyo Night theme + custom vars
|   |-- vite-env.d.ts               # Vite type declarations
|   |
|   |-- components/
|   |   |-- layout/
|   |   |   |-- Shell.tsx           # ** ENTRY: 3-panel resizable layout (repo | terminal | sidebar)
|   |   |   |-- TopBar.tsx          # Header with repo/branch + toggle buttons
|   |   |   |-- RepoSidebar.tsx     # Repo tree, worktree list, PR badges, branch tracking
|   |   |   |-- ChangesSidebar.tsx  # Git file status display (M/A/D/R/C)
|   |   |   |-- TeamSidebar.tsx     # Tasks, run timeline, agents, file grid, approvals
|   |   |   |-- TaskDashboard.tsx   # Cross-repo task overview (all repos, all worktrees)
|   |   |   +-- FileGrid.tsx        # Agent file activity heatmap (dot visualization)
|   |   |
|   |   |-- terminal/
|   |   |   |-- TerminalArea.tsx    # Multi-tab terminal container + agent activity
|   |   |   |-- TerminalView.tsx    # xterm.js wrapper (WebGL, fit, resize, input)
|   |   |   |-- TabBar.tsx          # Tab management, new terminal/claude dropdown
|   |   |   |-- AgentActivity.tsx   # Scrolling agent event log
|   |   |   |-- AgentBadge.tsx      # Mini status dot + current tool/file on tabs
|   |   |   +-- PresetManager.tsx   # Create/edit terminal presets (shell/claude)
|   |   |
|   |   |-- task/
|   |   |   |-- CreateTaskModal.tsx # New task form (issue-fix/pr-shepherd)
|   |   |   |-- TaskBadge.tsx       # Status dot with pulsing animation
|   |   |   |-- RunTimeline.tsx     # Run event history (steps, text, tools, cost)
|   |   |   +-- ApprovalDialog.tsx  # Permission approve/deny UI
|   |   |
|   |   |-- worktree/
|   |   |   |-- AddWorktreeModal.tsx    # Create worktree with live preview
|   |   |   |-- BranchWorktreeModal.tsx # Checkout existing branch as worktree
|   |   |   +-- DeleteWorktreeDialog.tsx # Confirmation + optional branch delete
|   |   |
|   |   |-- pr/
|   |   |   +-- PrTooltip.tsx       # Rich PR info tooltip (checks, reviews, state)
|   |   |
|   |   +-- ui/
|   |       +-- ContextMenu.tsx     # Generic right-click context menu
|   |
|   |-- lib/
|   |   |-- commands/               # Tauri IPC wrappers (never invoke() from components)
|   |   |   |-- git.ts              # 10 commands: repo CRUD, worktree CRUD, branches, status
|   |   |   |-- terminal.ts         # 4 commands: create/write/resize/close PTY sessions
|   |   |   |-- task.ts             # 6 commands: create/get/list tasks, watcher control
|   |   |   |-- run.ts              # 7 commands: launch/cancel/retry/resume/recover/permission
|   |   |   |-- agent.ts            # 5 commands: get agents/files, definitions, hooks
|   |   |   |-- github.ts           # 2 commands: PR status, GitHub availability check
|   |   |   |-- knowledge.ts        # 4 commands: query, ingest, stats, suggest
|   |   |   +-- workspace.ts        # 6 commands: app/repo config, presets
|   |   |
|   |   |-- stores/                 # SolidJS reactive stores (singleton factory pattern)
|   |   |   |-- repo.ts             # Repos, worktrees, branches, PR status, tracking
|   |   |   |-- task.ts             # Tasks, active run, run log, permissions
|   |   |   |-- terminal.ts         # Terminal tabs, sessions, output handlers
|   |   |   |-- agent.ts            # Agent status per tab, event log
|   |   |   |-- layout.ts           # Panel visibility, sidebar view state
|   |   |   +-- knowledge.ts        # Knowledge stats, loading state
|   |   |
|   |   |-- shortcuts.ts            # Keyboard shortcut registration
|   |   |-- utils.ts                # Pure helpers (parseArtifactSummary, statusColor, shortPath)
|   |   +-- __tests__/
|   |       +-- utils.test.ts       # Unit tests for utility functions
|   |
|   +-- types/                      # TypeScript type definitions (mirror Rust models)
|       |-- git.ts                  # RepoInfo, WorktreeInfo, BranchInfo, FileStatus, TrackingInfo
|       |-- task.ts                 # TaskStatus, TaskType, TaskScope, TaskFrontmatter, TaskInfo
|       |-- run.ts                  # RunStatus, RunInfo, RunStepEvent, PermissionRequestEvent
|       |-- terminal.ts             # PtyEvent, TabInfo
|       |-- agent.ts                # AgentEvent (discriminated union), AgentState, FileAccess
|       |-- github.ts               # PrInfo, CheckRunInfo, ReviewInfo
|       +-- knowledge.ts            # KnowledgeType, KnowledgeEntry, QueryResult, Suggestion
|
|-- src-tauri/                      # Rust backend (Tauri v2)
|   |-- Cargo.toml                  # Dependencies, feature gates (knowledge, sona)
|   |-- tauri.conf.json             # App config, bundled resources, security
|   |-- build.rs                    # Tauri build script
|   |-- capabilities/
|   |   +-- default.json            # Tauri v2 permissions
|   |
|   |-- src/
|   |   |-- main.rs                 # Tauri entry point
|   |   |-- lib.rs                  # ** ENTRY: app init, plugin setup, managed state, startup
|   |   |-- error.rs                # AppError enum (12 variants) via thiserror
|   |   |
|   |   |-- models/                 # Domain types (Serialize/Deserialize for IPC)
|   |   |   |-- mod.rs              # Re-exports
|   |   |   |-- repo.rs             # RepoInfo, WorktreeInfo, FileStatus, BranchInfo, PrInfo, TrackingInfo
|   |   |   |-- session.rs          # PtySession, PtyEvent, SessionId
|   |   |   |-- task.rs             # TaskStatus, TaskType, TaskScope, TaskFrontmatter, TaskInfo
|   |   |   |-- run.rs              # RunInfo, RunStatus, SidecarRequest/Response, LaunchOptions
|   |   |   |-- agent.rs            # Event (7 variants), AgentState, FileAccess, HookPayload
|   |   |   +-- knowledge.rs        # KnowledgeEntry, TrajectoryRecord, QueryResult, Suggestion
|   |   |
|   |   |-- commands/               # Thin IPC handlers (NO business logic)
|   |   |   |-- mod.rs
|   |   |   |-- git.rs              # 10 commands -> services/git
|   |   |   |-- terminal.rs         # 4 commands -> services/terminal
|   |   |   |-- task.rs             # 6 commands -> services/task + task_watcher
|   |   |   |-- run.rs              # 7 commands -> services/run_manager
|   |   |   |-- agent.rs            # 5 commands -> services/activity_store + agent_scanner
|   |   |   |-- github.rs           # 2 commands -> services/github
|   |   |   |-- workspace.rs        # 6 commands -> services/config
|   |   |   +-- knowledge.rs        # 4 commands -> services/knowledge (feature-gated)
|   |   |
|   |   +-- services/               # All business logic (daemon-extractable)
|   |       |-- mod.rs              # 23 service modules
|   |       |-- git.rs              # (514 LOC) Worktree CRUD, branches, status via git2
|   |       |-- terminal.rs         # (157 LOC) PTY session management via portable-pty
|   |       |-- task.rs             # (485 LOC) task.md parsing, YAML frontmatter, artifacts
|   |       |-- task_watcher.rs     # (227 LOC) File watcher with debounce, content-hash dedup
|   |       |-- run_manager.rs      # (717 LOC) Sidecar orchestration, run lifecycle
|   |       |-- run_effects.rs      # (222 LOC) Pure state machine (functional core)
|   |       |-- run_responses.rs    # (132 LOC) Thin wrappers around pure functions
|   |       |-- run_stale.rs        # (93 LOC) Stale detection, permission timeouts
|   |       |-- run_state.rs        # (125 LOC) Run persistence (.branchdeck/run.json)
|   |       |-- config.rs           # (150 LOC) Global/repo JSON config, hashed paths
|   |       |-- github.rs           # (469 LOC) octocrab PR/checks/reviews with caching
|   |       |-- event_bus.rs        # (32 LOC) tokio::broadcast pub/sub
|   |       |-- event_bridge.rs     # (29 LOC) EventBus -> Tauri frontend bridge
|   |       |-- activity_store.rs   # (297 LOC) Agent/file activity tracking with GC
|   |       |-- hook_receiver.rs    # (271 LOC) TCP listener for hook POSTs (port 13370)
|   |       |-- hook_config.rs      # (371 LOC) notify.sh generation, hook/MCP config mgmt
|   |       |-- agent_scanner.rs    # (258 LOC) .claude/agents/*.md frontmatter parsing
|   |       |-- knowledge.rs        # (627 LOC) RVF vector store, fastembed, SONA
|   |       |-- knowledge_ingestion.rs # (610 LOC) Trajectory capture, error pattern mining
|   |       |-- knowledge_query.rs  # (299 LOC) Vector search, re-ranking, metadata filters
|   |       |-- knowledge_merge.rs  # (262 LOC) Deduplication, pattern extraction
|   |       +-- knowledge_mcp.rs    # (319 LOC) MCP TCP endpoint for knowledge tools
|   |
|   +-- tests/                      # Integration + unit tests
|       |-- common/mod.rs           # Shared helpers: YAML fixtures, make_run_info()
|       |-- task_parsing.rs         # T1: parse_task_md, frontmatter manipulation
|       |-- artifact_capture.rs     # T2: git artifact capture with temp repos
|       |-- run_lifecycle.rs        # T4: pure state machine + persistence + stale detection
|       |-- git_operations.rs       # T6: worktree CRUD, branches, status
|       +-- agent_monitoring.rs     # T5: event bus, activity store, hook receiver
|
|-- docs/                           # Project documentation
|-- public/                         # Static assets
+-- config files                    # biome.json, tsconfig.json, vite.config.ts, vitest.config.ts
```

## Code Statistics

| Part | Files | Lines (approx) |
|------|-------|----------------|
| Rust services | 23 | ~6,700 |
| Rust commands | 9 | ~560 |
| Rust models | 7 | ~510 |
| Rust tests | 6 | ~600 |
| Rust other (lib, error, main) | 3 | ~500 |
| **Rust total** | **48** | **~8,870** |
| TS/TSX components | 17 | ~2,400 |
| TS stores | 6 | ~1,200 |
| TS commands | 8 | ~400 |
| TS types | 7 | ~220 |
| TS other | 5 | ~200 |
| **Frontend total** | **43** | **~4,420** |
| Sidecar (JS) | 2 | ~500 |
| CI/CD (YAML) | 5 | ~300 |
| **Project total** | **~98** | **~14,090** |

## Critical Folders

| Folder | Purpose |
|--------|---------|
| `src-tauri/src/services/` | All business logic - daemon-extractable, no Tauri imports in pure functions |
| `src-tauri/src/models/` | Domain types shared between backend and frontend via IPC |
| `src/lib/stores/` | Reactive state management - central source of frontend truth |
| `src/lib/commands/` | IPC boundary - only place `invoke()` is called |
| `sidecar/` | Bridge between Claude Agent SDK and Rust backend |
| `.github/workflows/` | CI/CD: checks, auto-review, auto-fix, release |

## Entry Points

| Entry | File | Purpose |
|-------|------|---------|
| Rust app | `src-tauri/src/lib.rs` | App init, plugin setup, managed state, startup recovery |
| Frontend | `src/App.tsx` -> `Shell.tsx` | 3-panel layout, keyboard shortcuts |
| Sidecar | `sidecar/agent-bridge.js` | Stdin/stdout protocol with RunManager |
| MCP | `sidecar/knowledge-mcp.js` | JSON-RPC tools for knowledge queries |
| Hook receiver | `services/hook_receiver.rs` | TCP:13370, receives Claude Code hook POSTs |
