use branchdeck_core::error::AppError;
use branchdeck_core::traits::EventEmitter;
use log::debug;

/// Daemon-side event emitter.
///
/// Lifecycle events from `run_effects::execute_effects` flow through here.
/// In the daemon, SSE delivery is handled by `EventBus` → `sse_handler`,
/// so this emitter logs for debugging. Additional transports (WebSocket push,
/// webhook) can be wired in here later.
pub struct DaemonEmitter;

impl EventEmitter for DaemonEmitter {
    fn emit_raw(&self, event: &str, _payload: serde_json::Value) -> Result<(), AppError> {
        debug!("DaemonEmitter: {event}");
        Ok(())
    }
}
