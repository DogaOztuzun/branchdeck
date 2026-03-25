use crate::error::AppError;

/// Transport-agnostic event emitter.
///
/// Implementations:
/// - `TauriEmitter` in branchdeck-desktop (wraps `AppHandle::emit()`)
/// - `DaemonEmitter` in branchdeck-daemon (future: `EventBus` → WebSocket)
pub trait EventEmitter: Send + Sync {
    /// Emit a named event with a pre-serialized JSON payload.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if the event cannot be emitted.
    fn emit_raw(&self, event: &str, payload: serde_json::Value) -> Result<(), AppError>;
}

/// Convenience: serialize `payload` to JSON and emit.
///
/// # Errors
///
/// Returns `AppError::Config` if serialization fails or the emitter rejects the event.
pub fn emit<T: serde::Serialize>(
    emitter: &dyn EventEmitter,
    event: &str,
    payload: &T,
) -> Result<(), AppError> {
    let value = serde_json::to_value(payload)
        .map_err(|e| AppError::Config(format!("Event serialize error: {e}")))?;
    emitter.emit_raw(event, value)
}
