---
description: Structured logging conventions for Rust services and frontend IPC
globs: ["**/*.rs", "**/*.ts"]
---

# Logging

## Rust Services — use `log` crate macros

- `info!()` — state-changing operations that succeed (create, delete, save)
- `debug!()` — read operations, expected branches (list, load, branch reuse)
- `error!()` — every failure path, including `.map_err()` on `?` propagation
- `trace!()` — hot-path diagnostics only (per-keystroke, per-frame). Never `debug!` on hot paths

```rust
pub fn create_thing(name: &str) -> Result<Thing, AppError> {
    let result = do_work(name).map_err(|e| {
        error!("Failed to create thing {name:?}: {e}");
        e
    })?;
    info!("Created thing {name:?} at {}", result.path.display());
    Ok(result)
}
```

## Frontend IPC Wrappers — wrap every `invoke()` with try/catch + `logError`

```typescript
import { error as logError } from '@tauri-apps/plugin-log';

export async function doThing(id: string): Promise<Thing> {
  try {
    return await invoke<Thing>('do_thing', { id });
  } catch (e) {
    logError(`doThing failed: ${e}`);
    throw e;
  }
}
```
