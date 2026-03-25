use crate::services::event_bus::EventBus;
use crate::traits::{self, EventEmitter};
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::broadcast::error::RecvError;

pub fn start(emitter: Arc<dyn EventEmitter>, event_bus: &EventBus) {
    let mut rx = event_bus.subscribe();

    tokio::spawn(async move {
        debug!("Event bridge started, forwarding EventBus to frontend");

        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Err(e) = traits::emit(emitter.as_ref(), "agent:event", &event) {
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
