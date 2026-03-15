# Branchdeck Tech Stack

## Overview

Branchdeck is a terminal-first desktop app for managing git repositories, worktrees, and coding agents. Built with Tauri v2 (Rust backend + SolidJS frontend), targeting Linux first.

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   Tauri Shell                    │
│                                                  │
│  ┌────────────┐  ┌──────────────┐  ┌──────────┐ │
│  │   Left     │  │   Center     │  │  Right   │ │
│  │  Sidebar   │  │   Terminal   │  │ Sidebar  │ │
│  │  (repos,   │  │   (xterm.js) │  │ (changes │ │
│  │  worktrees)│  │              │  │  files)  │ │
│  └────────────┘  └──────────────┘  └──────────┘ │
│                                                  │
├──────────────────────────────────────────────────┤
│              Tauri IPC boundary                  │
├──────────────────────────────────────────────────┤
│                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │   Git    │ │ Terminal │ │    App State      │ │
│  │  Module  │ │  Module  │ │    (config,       │ │
│  │  (git2)  │ │(portable │ │     repos,        │ │
│  │          │ │  -pty)   │ │     sessions)     │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
│                                                  │
│              Rust Backend (single crate)         │
└─────────────────────────────────────────────────┘
```

### Daemon-ready design

MVP runs as a single Tauri process. All backend logic lives behind a **service layer** (Rust traits) that the Tauri commands call into. This means:

- Tauri commands are thin wrappers — they call service methods and return results
- Business logic never imports Tauri types directly
- When it's time to add a daemon, the service layer moves into its own process and Tauri commands become RPC clients

```
MVP:        Tauri command → service layer → git2/pty/fs
Later:      Tauri command → RPC client → daemon → service layer → git2/pty/fs
```

The key rule: **no business logic in Tauri command handlers**.

---

## Frontend

### Core

| Package | Purpose |
|---|---|
| **SolidJS** | UI framework — fine-grained reactivity, no virtual DOM, ~7KB |
| **TypeScript** (strict mode) | All frontend code |
| **Tailwind CSS v4** | Styling |
| **@tauri-apps/api v2** | IPC with Rust backend |

### UI Components

| Package | Purpose |
|---|---|
| **@kobalte/core** | Accessible headless components (tabs, menus, dialogs, context menus) |
| **solid-resizable-panels** | Resizable pane layout (left/center/right split) |

### Terminal

| Package | Purpose |
|---|---|
| **@xterm/xterm** | Terminal emulator in the browser |
| **@xterm/addon-fit** | Auto-resize terminal to container |
| **@xterm/addon-webgl** | GPU-accelerated rendering |
| **@xterm/addon-search** | Search within terminal output |

We write our own thin SolidJS wrapper around xterm.js (~50 lines). No third-party wrapper needed.

### State Management

SolidJS signals and stores — no external state library. SolidJS's built-in reactivity is sufficient:

- `createSignal` for simple values (active tab, selected repo)
- `createStore` for nested objects (repo list, worktree state)
- `createResource` for async data from Tauri commands

### Build & Tooling

| Tool | Purpose |
|---|---|
| **Vite** | Frontend bundler (ships with Tauri template) |
| **Bun** | Package manager and script runner |
| **Biome** | Linting + formatting (replaces ESLint + Prettier) |

---

## Backend (Rust)

### Structure

Single crate for MVP, organized with modules:

```
src-tauri/
├── Cargo.toml
├── src/
│   ├── main.rs              # Tauri bootstrap
│   ├── commands/             # Tauri command handlers (thin wrappers)
│   │   ├── mod.rs
│   │   ├── git.rs
│   │   ├── terminal.rs
│   │   └── workspace.rs
│   ├── services/             # Business logic (daemon-extractable)
│   │   ├── mod.rs
│   │   ├── git.rs            # Git operations via git2
│   │   ├── terminal.rs       # PTY session management
│   │   └── workspace.rs      # Repo/worktree state
│   ├── models/               # Shared types and data structures
│   │   ├── mod.rs
│   │   ├── repo.rs
│   │   ├── worktree.rs
│   │   └── session.rs
│   └── error.rs              # App error types
```

### Dependencies

| Crate | Purpose |
|---|---|
| **tauri** v2 | Desktop app framework |
| **git2** | Git operations (clone, branch, worktree, diff, status) |
| **portable-pty** | Cross-platform PTY allocation |
| **serde** + **serde_json** | Serialization for IPC |
| **tokio** | Async runtime (Tauri uses this internally) |
| **notify** | OS-native filesystem watching |
| **thiserror** | Error type derivation |
| **dirs** | Platform-specific directory paths |

### What we intentionally skip for MVP

| Skip | Reason |
|---|---|
| **gix** (gitoxide) | git2 covers all MVP needs. Add later if performance demands pure-Rust git |
| **sqlx / sqlite** | File-based config (JSON/TOML) is enough for MVP |
| **octocrab** | Use `gh` CLI for GitHub operations instead of adding an API client |
| **tower / axum** | No HTTP server until daemon extraction |

---

## Development Environment

### Prerequisites

```bash
# System dependencies (Ubuntu 25.04)
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Bun
curl -fsSL https://bun.sh/install | bash

# Tauri CLI
cargo install tauri-cli --version "^2"
```

### Commands

```bash
# Install frontend dependencies
bun install

# Dev mode (hot reload frontend + Rust rebuild)
cargo tauri dev

# Production build
cargo tauri build

# Frontend only (no Rust)
bun run dev

# Lint + format
bun run check        # biome check
bun run check:fix    # biome check --fix

# Rust checks
cargo clippy --all-targets
cargo fmt --check
cargo test
```

---

## Code Standards

### TypeScript (Frontend)

**Biome config** (`biome.json`):

```json
{
  "$schema": "https://biomejs.dev/schemas/2.0.x/schema.json",
  "organizeImports": { "enabled": true },
  "linter": {
    "enabled": true,
    "rules": { "recommended": true }
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2,
    "lineWidth": 100
  },
  "javascript": {
    "formatter": {
      "quoteStyle": "single",
      "semicolons": "always"
    }
  }
}
```

Rules:
- **Strict TypeScript** — `strict: true` in tsconfig, no `any` except at IPC boundaries
- **Named exports only** — no default exports (better refactoring, explicit imports)
- **Components** — one component per file, filename matches component name
- **Signals naming** — prefix setters: `const [count, setCount] = createSignal(0)`
- **Tauri IPC calls** — wrap in dedicated functions under `src/lib/commands/`, never call `invoke()` directly from components
- **No barrel files** — import from the actual module, not `index.ts` re-exports

### Rust (Backend)

**Clippy** runs with default lints plus:

```toml
# Cargo.toml
[lints.rust]
unsafe_code = "deny"

[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
pedantic = { level = "warn", priority = -1 }
```

Rules:
- **No `unwrap()` / `expect()`** — use `?` operator with proper error types. Panics crash the whole app
- **No `unsafe`** — we don't need it, and it's easy to get wrong in a first Rust project
- **Error handling** — all errors flow through `thiserror` enums in `error.rs`, Tauri commands return `Result<T, AppError>`
- **Service layer purity** — services take dependencies as parameters (not global state), making them testable and daemon-extractable
- **Naming** — `snake_case` for functions/variables, `PascalCase` for types/structs, `SCREAMING_SNAKE` for constants (standard Rust conventions)
- **Module organization** — `mod.rs` re-exports the public API of each module. Keep internal types private

### Shared

- **Commit messages** — conventional commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`)
- **Branch naming** — `feat/short-description`, `fix/short-description`
- **No commented-out code** — delete it, git remembers
- **Dependencies** — justify new dependencies in the PR. Prefer small, focused crates/packages over large frameworks

---

## CI (GitHub Actions)

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  check-frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - run: bun install --frozen-lockfile
      - run: bun run check
      - run: bun run build

  check-rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: tauri-apps/tauri-action/setup@v0
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test
```

---

## File Structure (Full Project)

```
branchdeck/
├── src/                        # SolidJS frontend
│   ├── app.tsx                 # Root component
│   ├── index.tsx               # Entry point
│   ├── index.css               # Tailwind imports
│   ├── components/             # UI components
│   │   ├── layout/             # Shell, sidebars, panes
│   │   ├── terminal/           # xterm.js wrapper
│   │   ├── git/                # Repo list, worktree list, changes
│   │   └── common/             # Shared small components
│   ├── lib/
│   │   ├── commands/           # Tauri IPC wrappers
│   │   ├── stores/             # SolidJS stores (app state)
│   │   └── utils/              # Pure helper functions
│   └── types/                  # TypeScript type definitions
├── src-tauri/                  # Rust backend
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/           # Tauri v2 permissions
│   └── src/
│       ├── main.rs
│       ├── commands/
│       ├── services/
│       ├── models/
│       └── error.rs
├── biome.json
├── tsconfig.json
├── package.json
├── index.html
├── vite.config.ts
├── docs/
│   ├── mvp-brief.md
│   └── tech-stack.md           # This file
├── .github/
│   └── workflows/
│       └── ci.yml
├── CLAUDE.md
├── LICENSE                     # MIT
└── README.md
```

---

## Performance Budget

Targets for a tool that stays open all day:

| Metric | Target |
|---|---|
| Idle memory | < 80 MB |
| App startup | < 2 seconds |
| Terminal input latency | < 16ms (60fps) |
| Git status on large repo | < 500ms |
| Binary size | < 20 MB |

These are achievable with Tauri + Rust. If any metric regresses, investigate before adding features.

---

## Future Considerations (post-MVP)

These are noted for architecture awareness, not for implementation now:

- **Daemon extraction** — move `services/` into a standalone binary, add RPC layer (likely gRPC or Unix socket + JSON)
- **Web UI** — SolidJS frontend can serve from the daemon with minimal changes
- **MCP server** — expose workspace state to AI agents via Model Context Protocol
- **Multiple workspaces** — workspace switcher above repo tree, per-workspace config
- **macOS / Windows** — Tauri supports all three, but we test Linux first
