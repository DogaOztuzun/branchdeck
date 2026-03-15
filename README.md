<div align="center">

# Branchdeck

### Terminal-first workflow manager for git worktrees and coding agents

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat)](LICENSE)

**Linux-first** &nbsp;&bull;&nbsp; **Open Source** &nbsp;&bull;&nbsp; **~80MB RAM**

> **Warning**
> Branchdeck is in **alpha** — very early stage and under heavy development. Expect breaking changes, missing features, and rough edges. Contributions and feedback welcome!

</div>

<p align="center">
  <img src="docs/assets/branchdeck-alpha-screenshot.png" alt="Branchdeck screenshot" width="800" />
</p>

## Why Branchdeck?

Branchdeck is a lightweight desktop app that combines git worktree navigation, embedded terminals, and one-click coding agent sessions into a single three-pane layout.

Inspired by [Superset](https://github.com/superset-sh/superset) (terminal-first agent workflow) and [Arbor](https://github.com/penso/arbor) (native repo/worktree management), Branchdeck takes the best of both: Arbor's repo-centric shell with Superset's terminal-focused center pane — built on Tauri v2 instead of Electron or GPUI.

## Features

| Feature | Description |
|:--------|:------------|
| **Tabbed Terminals** | Shell and Claude Code tabs powered by xterm.js with WebGL rendering |
| **Git Worktree Management** | Add repos, browse worktrees, create new worktrees, see file status at a glance |
| **Agent Integration** | Launch Claude Code with agent teams support — more agents coming |
| **Three-Pane Layout** | Resizable repo sidebar, terminal center, changes sidebar — all collapsible |
| **Dark Theme** | Tokyo Night color scheme throughout |
| **Keyboard Driven** | Shortcuts for terminals, tabs, and sidebar toggles |
| **Config Persistence** | Window size, repo list, and active worktree restored on launch |
| **Lightweight** | Tauri v2 backend, no Electron, under 80MB RSS idle |

## Supported Agents

| Agent | Status |
|:------|:-------|
| [Claude Code](https://github.com/anthropics/claude-code) | Supported |
| Any CLI agent | Works in terminal tabs |

## Tech Stack

| Layer | Technology |
|:------|:-----------|
| **Desktop** | [Tauri v2](https://v2.tauri.app/) (Rust backend) |
| **Frontend** | [SolidJS](https://www.solidjs.com/) + [Tailwind CSS v4](https://tailwindcss.com/) |
| **Terminal** | [xterm.js](https://xtermjs.org/) + WebGL + portable-pty (Rust) |
| **Git** | [git2](https://docs.rs/git2) crate (in-process, no CLI shelling) |
| **Components** | [Kobalte](https://kobalte.dev/) + solid-resizable-panels |

## Requirements

| Requirement | Details |
|:------------|:--------|
| **OS** | Linux (Ubuntu 22.04+) |
| **Rust** | [rustup](https://rustup.rs/) stable |
| **Bun** | [bun.sh](https://bun.sh/) v1.0+ |
| **System libs** | See below |

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev
```

## Getting Started

```bash
# Install frontend dependencies
bun install

# Run in dev mode (hot reload + Rust rebuild)
bunx tauri dev

# Production build
bunx tauri build
```

## Development

```bash
bun run check              # Biome lint + format check
bun run check:fix          # Biome auto-fix
cargo clippy --all-targets # Rust linting (from src-tauri/)
cargo fmt --check          # Rust format check
cargo test                 # Rust tests
```

## Keyboard Shortcuts

| Shortcut | Action |
|:---------|:-------|
| `Ctrl+Shift+T` | New terminal tab |
| `Ctrl+Shift+A` | New Claude Code tab |
| `Ctrl+Shift+W` | Close active tab |
| `Ctrl+Shift+B` | Toggle repo sidebar |
| `Ctrl+Shift+L` | Toggle changes sidebar |

## Inspiration

Branchdeck draws from two excellent projects:

- **[Superset](https://github.com/superset-sh/superset)** — Turbocharged terminal for running parallel coding agents with worktree isolation, diff viewer, and workspace presets. Electron + React.
- **[Arbor](https://github.com/penso/arbor)** — Fully native desktop app for repositories, worktrees, embedded terminals, diffs, and PR context with a daemon-backed architecture. Rust + GPUI.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, branch strategy, code standards, and PR guidelines. All PRs target the `dev` branch.

## Development Methodology

This project uses the [BMAD-METHOD](https://github.com/bmadcode/BMAD-METHOD) for AI-assisted development workflow.

## License

MIT
