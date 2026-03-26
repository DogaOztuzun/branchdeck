---
description: Crate boundary rules for the 3-crate workspace architecture
globs: ["crates/**/*.rs", "src-tauri/**/*.rs"]
---

# Crate Boundaries

## Rule: If it has `use axum::` it goes in daemon. If it has `use tauri::` it goes in desktop. Everything else goes in core.

### branchdeck-core (library)
- All business logic: models, services, EventBus, error types
- NO `use axum::` imports
- NO `use tauri::` imports
- Use `write_atomic()` for all file persistence (never `std::fs::write`)
- Use `read_optional<T>()` for safe file reads (returns `Option<T>` for missing)

### branchdeck-daemon (binary)
- Axum server wrapping core services
- Route handlers MUST be thin: "Call core.X and return the result"
- If a handler has branching logic, that logic belongs in core
- Validation split: Serde validates shape, core validates semantics
- Error format: RFC 7807 Problem Details
- SSE event names: `namespace:snake_case_action` (e.g., `run:status_changed`)

### branchdeck-desktop (binary)
- Tauri thin shell: auto-launches daemon, connects via HTTP/SSE
- No business logic — only daemon lifecycle management + native chrome

## File Ownership (Single-Writer Pattern)

| File | Owner | No contention because |
|:--|:--|:--|
| `task.md` | task service | per-worktree, separate directories |
| `run.json` | RunManager | per-run, separate files |
| `learnings.yaml` | SAT scoring service | single writer |

No file locking needed. Parallel runs are in separate worktrees.

## CI Enforcement

```bash
! grep -r "use axum" crates/branchdeck-core/src/ || exit 1
! grep -r "use tauri" crates/branchdeck-core/src/ || exit 1
```
