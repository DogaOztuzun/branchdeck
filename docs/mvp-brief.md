Yes — the two layouts point to a good hybrid.

From your screenshots, **Superset’s strength is the “terminal-first, agent-in-focus” center pane**, while **Arbor’s strength is the “repo/worktree control + persistent side context” layout**. That matches their public positioning too: Superset describes itself as a turbocharged terminal for CLI coding agents with worktree isolation and built-in diff/review, while Arbor describes itself as a native app for repositories, issue-driven worktrees, embedded terminals, managed processes, diffs, and a daemon that also powers web UI, CLI, and MCP. ([GitHub][1])

My recommendation is:

## Product direction

Build the MVP as:

**Arbor-like shell + Superset-like center pane**

So the layout becomes:

* **Left sidebar:** repos, main repo context, worktrees
* **Center:** terminal-first main pane

  * shell tab
  * Claude Code tab
* **Right sidebar:** changes / files
* **Top strip:** current repo, branch, context, quick actions

That gives you the cleanest first version for your personal workflow.

## Why this hybrid is right

Your screenshot of Superset shows the value of:

* terminal occupying the main visual priority
* fast switching between agent tabs
* changes/files always near the working area

Your Arbor screenshot shows the value of:

* repository/worktree list staying visible
* compact, durable navigation model
* change list living on the right as contextual support

That hybrid also lines up with each tool’s stated product shape:

* Superset emphasizes workspaces, parallel agents, diff viewer, and quick context switching. ([GitHub][1])
* Arbor emphasizes repositories, worktrees, embedded terminals, PTY sessions, diffs, PR context, daemon-backed persistence, and web/CLI/MCP access. ([GitHub][2])

## For your MVP, I’d design these panes

### Left

Keep it Arbor-like:

* repos
* main repo context
* worktrees under repo
* add repo
* create worktree

### Center

Keep it Superset-like:

* terminal is the primary canvas
* Claude Code is just another terminal mode/tab, not a separate “chat app”
* later you can add Codex/OpenCode/Gemini tabs without redesign

### Right

Keep it compact:

* Changes
* Files
* later PR
* later Notes

This matches what both UIs are already doing visually, and it avoids a chat-heavy product drift.

## Future-proofing

Your future features fit this layout well:

### Multi-workspace/profile

This should sit **above** the repo tree, not inside the center pane.
So later:

* workspace switcher in top-left
* each workspace has its own repo list, GitHub identity, layout state, worktree root defaults

### Claude Code sandbox

This should be a **session mode**, not a separate app area.
Example later:

* shell
* Claude
* Claude sandboxed

### Web UI / remote usage

This is where Arbor’s direction is especially relevant. Arbor explicitly says its daemon powers the desktop app, web UI, CLI, and MCP server, and it supports remote authenticated daemons and remote worktrees over SSH. ([GitHub][2])

So if you want that path later, keep your architecture as:

* local core/service
* desktop UI client
* web UI client later

Not a desktop-only model.

## GH CLI

Yes, I would use `gh` from the start, but only as a helper, not as the foundation.

Superset already lists GitHub CLI as a requirement. ([GitHub][1])

Good uses for `gh` later:

* auth status
* repo metadata
* issue/PR lookup
* opening PR URLs
* possibly worktree-from-issue flows in V1.1+

But for MVP, don’t depend on `gh` for core repo/worktree behavior. Use regular `git` for:

* branch listing
* branch switching
* worktree creation
* diff/change status

Then add `gh` only where GitHub-specific workflows begin.

## What terminal are they using?

### Superset

I could not confirm it from the README alone, but there is strong evidence that Superset uses **xterm.js** in the desktop terminal layer:

* a release note says “Fix terminal pane spacing and remove xterm right gutter”
* an open issue mentions “terminal stream events” and batching before `xterm.write()`
* another issue explicitly says xterm.js lacks Kitty keyboard protocol support. ([GitHub][3])

So for practical purposes, Superset is using an **xterm.js-style terminal stack** in its Electron/TypeScript app. Superset’s repo also identifies its tech stack as Electron + React + Bun and labels the project as an Electron desktop app. ([GitHub][1])

### Arbor

Arbor is much clearer:

* it says it has a built-in PTY terminal with truecolor and `xterm-256color` support
* it has multiple terminal tabs per worktree
* it has an experimental embedded `libghostty-vt` engine, used by default when available
* it is built with Rust and GPUI. ([GitHub][2])

So Arbor is much more “native terminal stack” oriented, while Superset appears more “web-terminal in desktop shell.”

## What this means for your app

Since you chose Tauri and not Electron, I would recommend:

### For MVP

Use a **web terminal component** in the UI and keep process/session handling behind the native side.

That gives you:

* fast development
* terminal in the center pane
* easier Claude integration
* a path similar to Superset’s UX without Electron

### Later

If terminal fidelity becomes a real pain point, you can explore a more native terminal path, closer to Arbor’s philosophy.

So:

* **MVP:** Superset-style terminal experience
* **later upgrade path:** Arbor-style stronger native terminal backend

## My concrete recommendation

Build the UI model like this:

* **Top-left:** hidden single default workspace for MVP
* **Left sidebar:** repo tree, contexts, worktrees
* **Center:** terminal-first pane with tabs
* **Right sidebar:** changes/files
* **Bottom optional later:** ports/processes/logs

And architecturally:

* start desktop-first
* keep the backend/API boundary clean
* leave room for future web UI / remote daemon usage, inspired by Arbor’s daemon-backed model. ([GitHub][2])

So yes: **copy Superset’s center-pane feeling, copy Arbor’s outer-shell/navigation model, and keep GH CLI as a helper rather than a hard dependency.**

The next best step is to freeze:

* pane layout
* core objects
* Tauri command list
* terminal/session model

[1]: https://github.com/superset-sh/superset "GitHub - superset-sh/superset: IDE for the AI Agents Era - Run an army of Claude Code, Codex, etc. on your machine · GitHub"
[2]: https://github.com/penso/arbor "GitHub - penso/arbor: Run agentic coding workflows in a fully native desktop app for Git worktrees, terminals, and diffs. · GitHub"
[3]: https://github.com/superset-sh/superset/releases?utm_source=chatgpt.com "Releases · superset-sh/superset"
