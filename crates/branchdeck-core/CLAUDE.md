# branchdeck-core

Pure library crate. All business logic lives here. No I/O framework dependencies.

## Rules

- NO `use axum::` — belongs in daemon
- NO `use tauri::` — belongs in desktop
- Use `write_atomic()` for all file persistence — `std::fs::write` is blocked by clippy lint
- Use `read_optional<T>()` for reading files that may not exist
- All errors via `AppError` enum in `error.rs`
- Models derive `Serialize`, `Deserialize`, use `#[serde(rename_all = "kebab-case")]`

## Key Services

- `workflow_registry.rs` — discovery, validation, merge (embedded → global → project-local)
- `orchestrator.rs` — workflow-generic, accepts `&WorkflowDef`
- `run_manager.rs` — `HashMap<RunId, RunInfo>`, parallel runs, cost tracking
- `run_effects.rs` — pure `apply_*` functions returning `Vec<RunEffect>`
- `event_bus.rs` — event emission abstraction (no Tauri dependency)

## Testing

Tests in `tests/` directory. Pure function tests — no I/O, no mocks, fast.
See @.claude/rules/testing.md for effect pattern and conventions.
