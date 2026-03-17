# Branchdeck Feature Roadmap

Feature candidates sourced from Superset, Arbor, and original ideas.
Organized by priority tier.

## Currently Implemented

- Worktree create/delete with real-time preview modal
- Multi-tab terminals (shell + Claude Code)
- PTY via portable-pty + xterm.js WebGL
- Three-pane resizable layout (repo sidebar, terminal, changes sidebar)
- Git file status display (modified/added/deleted/renamed/conflicted)
- Session restoration (last repo, last worktree, window state)
- Per-repo config persistence

---

## High Priority

No architecture changes. All work within the current Tauri single-crate structure.

### Logging
Structured logging for frontend (TypeScript) and Rust backend. Log levels, file rotation, filterable output. Essential for debugging during development and diagnosing issues in production.

### Agent Status Indicators
Show real-time state of each agent tab: working, idle, waiting for input.
Color-coded dots on tabs + sidebar worktree entries.

### Agent/Terminal Presets
Pre-configured command templates. One-click launch like "Claude Code with prompt X" or "run dev server". Per-repo configurable.

### Create Worktree from Existing Branch
Additional tab in the create worktree modal for checking out an existing local/remote branch into a new worktree.

### Create Worktree from PR / Issue
Create a worktree directly from a GitHub PR URL or issue. Auto-name the worktree and branch from the PR/issue title.

### Branch Tracking (Ahead / Behind / PR / Issue)
Show how many commits a worktree's branch is ahead/behind its upstream. Auto-detect linked PRs and issues per branch — display PR status (open/merged/draft/review requested), CI check results, and linked issue state. Display in sidebar next to branch name.

### Issue Discovery
Auto-fetch open issues from GitHub/GitLab remotes. Display in sidebar. Create worktrees from issues.

---

## Medium-Top Priority

### Daemon Extraction
Move services/ into a standalone daemon process (Axum HTTP + WebSocket). Tauri commands become HTTP clients. Prerequisite for CLI companion and web dashboard.

### CLI Companion
Full CLI (`branchdeck-cli`) mirroring all app actions: repos, worktrees, terminals, agents, presets. Real-time sync with the running desktop app via shared daemon — changes from CLI reflect instantly in the UI and vice versa. Requires daemon extraction.

### Web Dashboard
SolidJS frontend served by the daemon over HTTPS with auth. Run Branchdeck on a small server, access from any browser. Full feature parity with desktop — orchestrate repos, worktrees, agents remotely. Requires daemon extraction.

### Agentic Orchestration Architecture
Core design principles for how Branchdeck manages agents and tasks:
- **Worktree-as-execution-unit** — each worktree is a self-contained execution context for one agent/task
- **Bring-your-own-agent** — pluggable agent support (Claude, Codex, Gemini, custom CLIs), not locked to one provider
- **Atomic task claiming** — tasks are claimed exclusively, no two agents work on the same task
- **Persistent agent/task state** — agent sessions and task progress survive app restarts and crashes
- **Heartbeat-driven continuation** — detect stalled/crashed agents and offer resume or reassignment
- **Goal alignment** — tasks carry context (issue, PR, description) so agents stay on track
- **Approval gates** — configurable checkpoints where agent work pauses for human review before continuing (e.g. before commit, before PR creation)

### Memory System
Global, workspace, and per-repository memory. Design the system so it can later support:
- Workspace memory (cross-repo context)
- Repo memory (repo-specific knowledge)
- Retrieval on session/task start (inject relevant memory when launching agents)
- Learning after task/review completion (capture insights from finished work)

Details TBD.

---

## Medium Priority

### Agent Notifications
Desktop notifications when an agent finishes, errors, or needs user attention (e.g. permission prompt).

### Stage / Unstage / Commit / Push
Full git workflow from the changes sidebar. Stage individual files, write commit message, push to remote with auto upstream setup.

### Diff Viewer
Side-by-side or inline diff display for changed files. Click a file in the changes sidebar to view its diff.

### Command Palette (Ctrl+K)
Searchable popup for quick access to any action: switch repo/worktree, run preset, create worktree, open settings.

### Setup / Teardown Scripts
Per-repo lifecycle hooks that run after worktree creation (e.g. `bun install`) and before deletion (e.g. cleanup). Configurable in repo settings.

### Multi-Agent CLI Support
Support launching other agent CLIs beyond Claude Code: Codex, Gemini, OpenCode. Configurable per-repo or global.

### Sandboxed Agent Execution (Bubblewrap)
Isolate agents per worktree using bubblewrap (bwrap). Bind-mount worktree read-write, system tools read-only, optional network isolation. Redundant for Claude Code (has its own bwrap sandbox), but needed for agents without built-in sandboxing. Ship alongside multi-agent CLI support.

### Built-in Browser Pane (LightPanda)
Embedded lightweight browser (LightPanda) for viewing running dev apps. CDP server for agent-accessible console logs, network traffic, screenshots, JS evaluation. Needs validation: LightPanda CDP + MCP bridge compatibility is unproven.

### Port Detection & Monitoring
Auto-detect open ports per worktree (dev servers, etc). Show in UI with labels. Click to open in browser or kill process.

### Open in External Editor
Right-click worktree or file to open in VS Code, Cursor, or other configured editor.

### Branch Prefix Modes
Configurable branch naming: `none`, `git-author/`, `github-user/`, or custom prefix. Per-repo setting.

---

## Lower Priority

### MCP Server
Expose workspace state (repos, worktrees, terminals, agents) to AI tools via Model Context Protocol.

### Remote Worktrees (SSH)
Create and manage worktrees on remote machines over SSH. Terminal sessions via remote PTY.

### Task Scheduling
Cron-style scheduled tasks per repo. Run tests, fetch upstream, or trigger agents on a schedule.

### Theme System
Multiple color themes beyond Tokyo Night. Theme picker in settings.

### Keyboard Shortcut Customization
User-configurable keybindings with import/export.

### Built-in File Editor
View and edit files with syntax highlighting inside Branchdeck, without needing an external editor.

### Notification Webhooks
Send agent lifecycle events to Slack, Discord, or custom HTTP endpoints.

### Process Management (Procfile)
Manage long-running processes (dev server, database, etc) per worktree. Start/stop/restart with status tracking.
