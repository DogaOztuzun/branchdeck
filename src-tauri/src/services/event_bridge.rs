use crate::services::event_bus::EventBus;
use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter};
use tokio::sync::broadcast::error::RecvError;

pub fn start(app_handle: AppHandle, event_bus: &EventBus) {
    let mut rx = event_bus.subscribe();

    tauri::async_runtime::spawn(async move {
        debug!("Event bridge started, forwarding EventBus to frontend");

        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = app_handle.emit("agent:event", &event) {
                        error!("Failed to emit agent:event to frontend: {e}");
                    }
                }
                Err(RecvError::Lagged(count)) => {
                    warn!("Event bridge lagged, skipped {count} events");
                }
                Err(RecvError::Closed) => {
                    info!("Event bus closed, stopping event bridge");
                    break;
                }
            }
        }
    });
}
