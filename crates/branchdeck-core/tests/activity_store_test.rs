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

#[tokio::test]
async fn persistence_round_trip_survives_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // Create a store with persistence, publish events via bus
    {
        let event_bus = Arc::new(EventBus::new());
        let store = Arc::new(
            ActivityStore::new_with_persistence(&data_dir).unwrap(),
        );
        store.start_subscriber(&event_bus);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        event_bus.publish(Event::SessionStart {
            session_id: "persist-sess".into(),
            tab_id: "tab-p".into(),
            model: None,
            ts: 5000,
        });

        event_bus.publish(Event::ToolEnd {
            session_id: "persist-sess".into(),
            agent_id: None,
            tab_id: "tab-p".into(),
            tool_name: "Edit".into(),
            tool_use_id: "tu-p1".into(),
            file_path: Some("/src/app.rs".into()),
            ts: 5100,
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify events are queryable
        let events = store.get_events_since(0).await;
        assert_eq!(events.len(), 2);
    }

    // Simulate restart: create a new store from the same directory
    let store2 = ActivityStore::new_with_persistence(&data_dir).unwrap();

    // Loaded events should be queryable
    let events = store2.get_events_since(0).await;
    assert_eq!(events.len(), 2);

    // Agent state should be rebuilt from replayed events
    let agents = store2.get_agents_for_session("persist-sess").await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].session_id, "persist-sess");

    // File access should be rebuilt
    let files = store2.get_files_for_session("persist-sess").await;
    assert_eq!(files.len(), 1);
    assert!(files[0].was_modified);
}

#[tokio::test]
async fn loaded_events_appear_in_time_filtered_queries() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().to_path_buf();

    // Populate events at different timestamps
    {
        let event_bus = Arc::new(EventBus::new());
        let store = Arc::new(
            ActivityStore::new_with_persistence(&data_dir).unwrap(),
        );
        store.start_subscriber(&event_bus);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        event_bus.publish(Event::SessionStart {
            session_id: "old-sess".into(),
            tab_id: "tab-1".into(),
            model: None,
            ts: 1000,
        });

        event_bus.publish(Event::SessionStart {
            session_id: "new-sess".into(),
            tab_id: "tab-2".into(),
            model: None,
            ts: 9000,
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Reload and query with time filter
    let store2 = ActivityStore::new_with_persistence(&data_dir).unwrap();

    let all = store2.get_events_since(0).await;
    assert_eq!(all.len(), 2);

    let recent = store2.get_events_since(5000).await;
    assert_eq!(recent.len(), 1);

    let none = store2.get_events_since(99_999).await;
    assert!(none.is_empty());
}
