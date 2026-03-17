//! Integration tests for the agent monitoring pipeline.
//!
//! Tests the full flow: hook receiver → event bus → activity store,
//! plus hook config manager and agent scanner.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use branchdeck_lib::models::agent::{AgentStatus, Event};
use branchdeck_lib::services::activity_store::ActivityStore;
use branchdeck_lib::services::agent_scanner;
use branchdeck_lib::services::event_bus::EventBus;
use branchdeck_lib::services::hook_config;
use branchdeck_lib::services::hook_receiver;

/// Test: EventBus pub/sub delivers events to subscribers.
#[tokio::test]
async fn event_bus_pubsub() {
    let bus = EventBus::new();
    let mut rx = bus.subscribe();

    let event = Event::SessionStart {
        session_id: "sess-1".into(),
        tab_id: "tab-1".into(),
        model: Some("opus".into()),
        ts: 1000,
    };

    bus.publish(event.clone());

    let received = tokio::time::timeout(Duration::from_millis(100), rx.recv())
        .await
        .unwrap()
        .unwrap();

    if let Event::SessionStart { session_id, .. } = received {
        assert_eq!(session_id, "sess-1");
    } else {
        panic!("Expected SessionStart event");
    }
}

/// Test: ActivityStore tracks agent state through session lifecycle.
#[tokio::test]
async fn activity_store_lifecycle() {
    let bus = EventBus::new();
    let store = Arc::new(ActivityStore::new());
    store.start_subscriber(&bus);

    // Give subscriber time to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Session start
    bus.publish(Event::SessionStart {
        session_id: "sess-1".into(),
        tab_id: "tab-1".into(),
        model: None,
        ts: 1000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].status, AgentStatus::Active);
    assert_eq!(agents[0].session_id, "sess-1");

    // Tool start
    bus.publish(Event::ToolStart {
        session_id: "sess-1".into(),
        agent_id: None,
        tab_id: "tab-1".into(),
        tool_name: "Read".into(),
        tool_use_id: "tu-1".into(),
        file_path: Some("/src/main.rs".into()),
        ts: 2000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    assert_eq!(agents[0].current_tool.as_deref(), Some("Read"));
    assert_eq!(agents[0].current_file.as_deref(), Some("/src/main.rs"));

    let files = store.get_all_files().await;
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "/src/main.rs");
    assert_eq!(files[0].access_count, 1);
    assert!(!files[0].was_modified);

    // Tool end (Write marks file as modified)
    bus.publish(Event::ToolEnd {
        session_id: "sess-1".into(),
        agent_id: None,
        tab_id: "tab-1".into(),
        tool_name: "Write".into(),
        tool_use_id: "tu-2".into(),
        file_path: Some("/src/main.rs".into()),
        ts: 3000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    assert_eq!(agents[0].status, AgentStatus::Idle);
    assert!(agents[0].current_tool.is_none());

    // Session stop
    bus.publish(Event::SessionStop {
        session_id: "sess-1".into(),
        tab_id: "tab-1".into(),
        ts: 4000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    assert_eq!(agents[0].status, AgentStatus::Stopped);
}

/// Test: Subagent tracking (separate key from main agent).
#[tokio::test]
async fn activity_store_subagents() {
    let bus = EventBus::new();
    let store = Arc::new(ActivityStore::new());
    store.start_subscriber(&bus);
    tokio::time::sleep(Duration::from_millis(10)).await;

    bus.publish(Event::SessionStart {
        session_id: "sess-1".into(),
        tab_id: "tab-1".into(),
        model: None,
        ts: 1000,
    });

    bus.publish(Event::SubagentStart {
        session_id: "sess-1".into(),
        agent_id: "sub-1".into(),
        agent_type: "Explore".into(),
        tab_id: "tab-1".into(),
        ts: 2000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    assert_eq!(agents.len(), 2, "Should have main + subagent");

    let sub = agents.iter().find(|a| a.agent_id.is_some()).unwrap();
    assert_eq!(sub.agent_id.as_deref(), Some("sub-1"));
    assert_eq!(sub.agent_type.as_deref(), Some("Explore"));
    assert_eq!(sub.status, AgentStatus::Active);

    bus.publish(Event::SubagentStop {
        session_id: "sess-1".into(),
        agent_id: "sub-1".into(),
        agent_type: "Explore".into(),
        tab_id: "tab-1".into(),
        ts: 3000,
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let agents = store.get_agents_for_tab("tab-1").await;
    let sub = agents.iter().find(|a| a.agent_id.is_some()).unwrap();
    assert_eq!(sub.status, AgentStatus::Stopped);
}

/// Test: Hook receiver accepts POST /hook and publishes events.
#[tokio::test]
async fn hook_receiver_end_to_end() {
    let bus = Arc::new(EventBus::new());
    let mut rx = bus.subscribe();

    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let receiver_bus = Arc::clone(&bus);

    // Use a random high port to avoid conflicts
    let port = 13_399;
    tokio::spawn(async move {
        hook_receiver::start(receiver_bus, port, ready_tx).await;
    });

    ready_rx.await.unwrap().unwrap();

    // Send a SessionStart hook payload via HTTP
    let payload = serde_json::json!({
        "session_id": "test-sess",
        "hook_event_name": "SessionStart",
        "model": "sonnet",
        "branchdeck_tab_id": "tab-42"
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://127.0.0.1:{port}/hook"))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Verify event was published
    let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .unwrap()
        .unwrap();

    if let Event::SessionStart {
        session_id,
        tab_id,
        model,
        ..
    } = event
    {
        assert_eq!(session_id, "test-sess");
        assert_eq!(tab_id, "tab-42");
        assert_eq!(model.as_deref(), Some("sonnet"));
    } else {
        panic!("Expected SessionStart, got {event:?}");
    }

    // Send a PreToolUse with file path extraction
    let tool_payload = serde_json::json!({
        "session_id": "test-sess",
        "hook_event_name": "PreToolUse",
        "tool_name": "Read",
        "tool_input": { "file_path": "/src/lib.rs" },
        "tool_use_id": "tu-abc",
        "branchdeck_tab_id": "tab-42"
    });

    let resp = client
        .post(format!("http://127.0.0.1:{port}/hook"))
        .json(&tool_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let event = tokio::time::timeout(Duration::from_millis(500), rx.recv())
        .await
        .unwrap()
        .unwrap();

    if let Event::ToolStart {
        tool_name,
        file_path,
        ..
    } = event
    {
        assert_eq!(tool_name, "Read");
        assert_eq!(file_path.as_deref(), Some("/src/lib.rs"));
    } else {
        panic!("Expected ToolStart, got {event:?}");
    }
}

/// Test: Hook receiver rejects non-POST and oversized payloads.
#[tokio::test]
async fn hook_receiver_rejects_bad_requests() {
    let bus = Arc::new(EventBus::new());
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();

    let port = 13_398;
    let receiver_bus = Arc::clone(&bus);
    tokio::spawn(async move {
        hook_receiver::start(receiver_bus, port, ready_tx).await;
    });
    ready_rx.await.unwrap().unwrap();

    let client = reqwest::Client::new();

    // GET should be rejected
    let resp = client
        .get(format!("http://127.0.0.1:{port}/hook"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Wrong path should be rejected
    let resp = client
        .post(format!("http://127.0.0.1:{port}/other"))
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

/// Test: notify.sh generation and hook install/remove.
#[test]
fn hook_config_install_remove() {
    let tmp = std::env::temp_dir().join("branchdeck_test_hooks");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    let script_path = tmp.join("notify.sh");
    std::fs::write(&script_path, "#!/bin/bash\necho test").unwrap();

    let repo_path = tmp.join("repo");
    std::fs::create_dir_all(&repo_path).unwrap();

    let repo_str = repo_path.to_str().unwrap();

    // Install hooks
    hook_config::install_hooks(repo_str, &script_path).unwrap();

    // Verify settings.json was created
    let settings_path = repo_path.join(".claude/settings.json");
    assert!(settings_path.exists());

    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();

    let hooks = settings.get("hooks").unwrap().as_object().unwrap();
    assert!(hooks.contains_key("SessionStart"));
    assert!(hooks.contains_key("PreToolUse"));
    assert!(hooks.contains_key("PostToolUse"));
    assert!(hooks.contains_key("Stop"));
    assert_eq!(hooks.len(), 7);

    // Verify new format: each entry has matcher + hooks array
    let session_arr = hooks["SessionStart"].as_array().unwrap();
    assert_eq!(session_arr.len(), 1);
    let entry = &session_arr[0];
    assert_eq!(entry["matcher"].as_str(), Some(""));
    assert!(entry["hooks"].as_array().unwrap()[0]["command"]
        .as_str()
        .unwrap()
        .contains("notify.sh"));

    // Verify idempotency — install again, no duplicates
    hook_config::install_hooks(repo_str, &script_path).unwrap();
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    let arr = settings["hooks"]["SessionStart"].as_array().unwrap();
    assert_eq!(arr.len(), 1, "Should not duplicate hook entries");

    // Remove hooks
    hook_config::remove_hooks(repo_str, &script_path).unwrap();

    // Settings file should exist but have no hooks key
    let settings: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    assert!(
        settings.get("hooks").is_none(),
        "hooks key should be removed"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Test: Agent scanner with real .md files.
#[test]
fn agent_scanner_real_files() {
    let tmp = std::env::temp_dir().join("branchdeck_test_scanner");
    let _ = std::fs::remove_dir_all(&tmp);
    let agents_dir = tmp.join(".claude/agents");
    std::fs::create_dir_all(&agents_dir).unwrap();

    std::fs::write(
        agents_dir.join("researcher.md"),
        "---\nname: researcher\ndescription: \"Deep research agent\"\nmodel: opus\ntools: [\"Read\", \"Grep\", \"Glob\"]\npermission_mode: plan\n---\n\nYou are a research agent.",
    ).unwrap();

    std::fs::write(
        agents_dir.join("fixer.md"),
        "---\nname: fixer\ndescription: Bug fix agent\ntools: [\"Read\", \"Edit\"]\n---\n\nFix bugs.",
    ).unwrap();

    // Non-.md file should be ignored
    std::fs::write(agents_dir.join("notes.txt"), "not an agent").unwrap();

    let defs = agent_scanner::scan_agent_definitions(tmp.to_str().unwrap()).unwrap();
    assert_eq!(defs.len(), 2);

    let researcher = defs.iter().find(|d| d.name == "researcher").unwrap();
    assert_eq!(researcher.description, "Deep research agent");
    assert_eq!(researcher.model.as_deref(), Some("opus"));
    assert_eq!(researcher.tools, vec!["Read", "Grep", "Glob"]);
    assert_eq!(researcher.permission_mode.as_deref(), Some("plan"));

    let fixer = defs.iter().find(|d| d.name == "fixer").unwrap();
    assert_eq!(fixer.description, "Bug fix agent");
    assert!(fixer.model.is_none());
    assert_eq!(fixer.tools, vec!["Read", "Edit"]);

    let _ = std::fs::remove_dir_all(&tmp);
}

/// Test: ensure_notify_script creates an executable script.
#[test]
fn notify_script_creation() {
    let result = hook_config::ensure_notify_script();
    assert!(result.is_ok());

    let path = result.unwrap();
    assert!(path.exists());

    let contents = std::fs::read_to_string(&path).unwrap();
    assert!(contents.contains("BRANCHDECK_PORT"));
    assert!(contents.contains("BRANCHDECK_TAB_ID"));
    assert!(contents.contains("curl"));

    // Check executable permission
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert!(mode & 0o111 != 0, "Script should be executable");
}
