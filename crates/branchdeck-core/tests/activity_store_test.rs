#![allow(clippy::unwrap_used, clippy::expect_used, unused_must_use)]

use branchdeck_core::models::agent::Event;
use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use std::sync::Arc;

async fn create_store_with_bus() -> (Arc<ActivityStore>, Arc<EventBus>) {
    let event_bus = Arc::new(EventBus::new());
    let activity_store = Arc::new(ActivityStore::new());
    activity_store.start_subscriber(&event_bus);
    // Small delay to let subscriber spawn
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    (activity_store, event_bus)
}

#[tokio::test]
async fn get_agents_for_session_returns_matching_agents() {
    let (store, bus) = create_store_with_bus().await;

    bus.publish(Event::SessionStart {
        session_id: "sess-1".into(),
        tab_id: "tab-1".into(),
        model: None,
        ts: 1000,
    });

    bus.publish(Event::SubagentStart {
        session_id: "sess-1".into(),
        agent_id: "sub-1".into(),
        agent_type: "task".into(),
        tab_id: "tab-1".into(),
        ts: 1100,
    });

    // Different session
    bus.publish(Event::SessionStart {
        session_id: "sess-2".into(),
        tab_id: "tab-2".into(),
        model: None,
        ts: 1200,
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let agents = store.get_agents_for_session("sess-1").await;
    assert_eq!(agents.len(), 2); // main session + subagent

    let agents2 = store.get_agents_for_session("sess-2").await;
    assert_eq!(agents2.len(), 1);

    // Non-existent session
    let agents_empty = store.get_agents_for_session("sess-999").await;
    assert!(agents_empty.is_empty());
}

#[tokio::test]
async fn get_files_for_session_returns_matching_files() {
    let (store, bus) = create_store_with_bus().await;

    bus.publish(Event::SessionStart {
        session_id: "sess-a".into(),
        tab_id: "tab-a".into(),
        model: None,
        ts: 1000,
    });

    bus.publish(Event::ToolStart {
        session_id: "sess-a".into(),
        agent_id: None,
        tab_id: "tab-a".into(),
        tool_name: "Read".into(),
        tool_use_id: "tu-1".into(),
        file_path: Some("/src/main.rs".into()),
        ts: 1100,
    });

    bus.publish(Event::ToolEnd {
        session_id: "sess-a".into(),
        agent_id: None,
        tab_id: "tab-a".into(),
        tool_name: "Edit".into(),
        tool_use_id: "tu-2".into(),
        file_path: Some("/src/lib.rs".into()),
        ts: 1200,
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let files = store.get_files_for_session("sess-a").await;
    assert_eq!(files.len(), 2);

    // File modified by Edit should have was_modified = true
    let lib_file = files.iter().find(|f| f.path == "/src/lib.rs").unwrap();
    assert!(lib_file.was_modified);

    let main_file = files.iter().find(|f| f.path == "/src/main.rs").unwrap();
    assert!(!main_file.was_modified);
}

#[tokio::test]
async fn get_active_sessions_filters_correctly() {
    let (store, bus) = create_store_with_bus().await;

    bus.publish(Event::SessionStart {
        session_id: "active-1".into(),
        tab_id: "tab-1".into(),
        model: None,
        ts: 1000,
    });

    bus.publish(Event::SessionStart {
        session_id: "stopped-1".into(),
        tab_id: "tab-2".into(),
        model: None,
        ts: 1100,
    });

    bus.publish(Event::SessionStop {
        session_id: "stopped-1".into(),
        tab_id: "tab-2".into(),
        ts: 1200,
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let active = store.get_active_sessions().await;
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].session_id, "active-1");

    let all = store.get_all_agents().await;
    assert_eq!(all.len(), 2);
}
