# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project Overview

Branchdeck is a daemon-first autonomous development system — desktop-delivered, local-first. Detects quality issues, implements fixes, verifies improvements, learns from results.

**Stack:** Tauri v2 (Rust) + SolidJS (TypeScript) + xterm.js (terminal) + Axum (daemon)

## Critical Rules

- **Package manager is Bun** — NEVER use `npm`, `npx`, `yarn`, or `pnpm`
- **Linter is Biome** — NEVER use `eslint`, `prettier`, or `tsc` for linting/formatting
- **Dev server:** `bunx tauri dev` (NOT `cargo tauri dev`)
- **Build:** `bunx tauri build`
- **Verify before commit:** `bun run verify`

## Commands

```bash
bun install                    # Install frontend + sidecar deps (workspaces)
bunx tauri dev                 # Dev mode (hot reload + Rust rebuild)
bunx tauri build               # Production build
bun run check                  # Biome lint + format check
bun run check:fix              # Biome auto-fix
bun test                       # Frontend tests (vitest)
bun run verify                 # Full check suite (Biome + vitest + fmt + clippy + cargo test)
```

## Architecture (3-Crate Workspace)

```
crates/
  branchdeck-core/       # Library: all business logic (models, services, EventBus)
  branchdeck-daemon/     # Binary: Axum server + CLI subcommands
  branchdeck-desktop/    # Binary: Tauri thin shell (auto-launches daemon)
src/                     # Frontend: SolidJS (shared desktop + web)
sidecar/                 # Agent bridge: Node.js + Claude Agent SDK
workflows/               # Default workflow definitions (YAML + SKILL.md)
```

**Crate boundary rule:** `use axum::` → daemon. `use tauri::` → desktop. Everything else → core.
See @.claude/rules/crate-boundaries.md for details.

### Frontend (`src/`)
- **Framework:** SolidJS + Tailwind CSS v4 + Kobalte + xterm.js
- **API client:** `src/lib/api/client.ts` (ky) — never call fetch/ky directly from components
- **SSE events:** `src/lib/api/events.ts` — `onEvent<T>()` helper
- **Stores:** `src/lib/stores/` — factory functions, createStore + produce
- **Components:** `src/components/{category}/ComponentName.tsx`

### Backend (`crates/branchdeck-core/src/`)
- **Services:** all business logic — pure functions, return effects
- **Models:** domain types with Serialize/Deserialize
- **Errors:** single `AppError` enum via thiserror
- **Key rule:** No business logic in command/route handlers — call a service, return the result

## Code Standards

### TypeScript
- Strict mode, no `any` except API boundaries
- Named exports only, no barrel files
- Single quotes, semicolons, 2-space indent, 100 char width (Biome enforces)

### Rust
- `unsafe` denied, `unwrap()`/`expect()` warned — use `?` with thiserror
- Clippy pedantic enabled
- All errors via `AppError` enum in `error.rs`
- Models: `#[serde(rename_all = "kebab-case")]`
- File writes: `write_atomic()` only — `std::fs::write` blocked by clippy lint

### Conventions
- Conventional commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`)
- No commented-out code
- Structured logging — see @.claude/rules/logging.md

## Adding a New Feature

1. Models in `crates/branchdeck-core/src/models/`
2. Service in `crates/branchdeck-core/src/services/` — pure functions, return effects
3. Tests in `crates/branchdeck-core/tests/`
4. Route in `crates/branchdeck-daemon/src/routes/` — thin handler
5. Frontend types in `src/types/`
6. API wrapper in `src/lib/api/` or `src/lib/commands/`
7. Store in `src/lib/stores/` if reactive state needed
8. Component in `src/components/{category}/`
9. **Lifecycle trace** — walk one scenario through every function call before committing
10. **Verify:** `bun run verify`

## Testing

See @.claude/rules/testing.md for full testing patterns and effect executor rules.

**Quick reference:**
- Pure functions first, return effects, no AppHandle in business logic
- Test: happy path + one error + one edge case. Don't over-test.
- Effect executor arms MUST do real work — no no-op match arms

## Design System

Always read @DESIGN.md before any visual or UI decisions.
Tokyo Night palette, JetBrains Mono, 0px border-radius, dark-only.
Do not deviate without explicit user approval.

## CI/CD

### Workflows (`.github/workflows/`)
- **ci.yml** — PR checks: Biome + Clippy + fmt → build + tests
- **claude-code-review.yml** — Auto-reviews every PR
- **claude.yml** — Responds to `@claude` mentions
- **claude-ci-fix.yml** — Auto-fixes CI failures on feature branches
- **release.yml** — Manual dispatch: version bump → tag → build → release

## Key Docs

- `DESIGN.md` — design system (typography, color, spacing, motion, layout)
- `_bmad-output/planning-artifacts/prd.md` — product requirements (59 FRs, 26 NFRs)
- `_bmad-output/planning-artifacts/architecture.md` — architecture decisions, implementation patterns
- `_bmad-output/planning-artifacts/epics.md` — 9 epics, 43 stories with ACs
- `_bmad-output/project-context.md` — AI-optimized codebase context

## Detailed Rules (loaded on demand)

- @.claude/rules/testing.md — effect pattern, test conventions, what's not tested
- @.claude/rules/logging.md — structured logging for Rust and TypeScript
- @.claude/rules/crate-boundaries.md — 3-crate workspace rules, file ownership
- @.claude/rules/daemon-api.md — REST API, SSE, frontend transport patterns
- @.claude/rules/workflows.md — WorkflowDef schema, authoring, registry merge
