---
description: Testing patterns, effect executor rules, and test file conventions
globs: ["**/*.rs", "**/*.ts", "**/*.test.ts"]
---

# Testing

## Pattern: Functional Core / Imperative Shell

- **Pure `apply_*` functions** — take state + inputs, return `(mutated_state, Vec<RunEffect>)`. No I/O, no async.
- **`RunEffect` enum** — flat list of side effects as data. One unit of work per variant.
- **`execute_effects()`** — thin executor, one line per match arm. Tauri-coupled, trivial by design.
- **Thin wrappers** — call `apply_*`, then `execute_effects`. Glue code only.

## Designing for Testability

1. Pure functions first — pass `now_ms` instead of calling `now_epoch_ms()`. Pass data instead of reading files.
2. Return effects, don't execute them — use `Vec<RunEffect>`. Add new variants as needed.
3. No `AppHandle` in business logic — belongs in executor and thin wrappers only.
4. No `now_epoch_ms()` inside pure functions — pass time as parameter.
5. Keep `RunEffect` enum flat — one line per executor arm. Group into sub-enums past 20 variants.

## Effect Executor Completeness (CRITICAL)

1. **No no-op match arms** — every arm must do real work. Log-only arms are bugs.
2. **Don't fabricate IDs** — read IDs from the component that generates them.
3. **Trace full lifecycle before committing** — walk one scenario through every function call.
4. **Checklist each effect variant** — one line per variant describing what it must do.
5. **Shared helpers for shared logic** — divergent implementations of same logic = bugs.

## Test Conventions

- Shared helpers in `tests/common/mod.rs`
- Use `tempfile::TempDir` for filesystem tests, `git2::Repository` for git tests
- Assert effects with `.iter().any(|e| matches!(e, ...))` (order-independent)
- `#![allow(clippy::unwrap_used, clippy::expect_used)]` in test files
- Test: happy path + one error case + one edge case per function. Don't over-test.

## What's NOT Tested

| Area | Reason |
|:--|:--|
| `commands/` (Tauri IPC handlers) | Require full Tauri runtime |
| `run_manager.rs` (process orchestration) | Holds child processes, Arc<Mutex> |
| `terminal.rs` (PTY management) | Needs real terminal |
| `knowledge*.rs` (embedding + vector) | Needs ONNX model download |
