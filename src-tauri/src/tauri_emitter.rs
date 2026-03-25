use branchdeck_core::error::AppError;
use branchdeck_core::traits::EventEmitter;
use tauri::{AppHandle, Emitter};

/// Tauri implementation of `EventEmitter` — forwards events to the frontend via IPC.
pub struct TauriEmitter(AppHandle);

impl TauriEmitter {
    pub fn new(handle: AppHandle) -> Self {
        Self(handle)
    }
}

impl EventEmitter for TauriEmitter {
    fn emit_raw(&self, event: &str, payload: serde_json::Value) -> Result<(), AppError> {
        self.0
            .emit(event, payload)
            .map_err(|e| AppError::Emit(format!("Tauri emit error: {e}")))
    }
}
