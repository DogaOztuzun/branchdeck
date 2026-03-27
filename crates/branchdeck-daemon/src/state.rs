use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::services::run_manager::RunManagerState;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub event_bus: Arc<EventBus>,
    pub activity_store: Arc<ActivityStore>,
    pub run_manager: RunManagerState,
}
