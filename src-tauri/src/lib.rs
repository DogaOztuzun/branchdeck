mod commands;
mod error;
pub mod models;
pub mod services;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

const HOOK_RECEIVER_PORT: u16 = 13_370;
const ACTIVITY_GC_TTL_MS: u64 = 300_000; // 5 minutes

fn init_agent_config() -> commands::agent::AgentMonitorConfig {
    match services::hook_config::ensure_notify_script() {
        Ok(script_path) => {
            log::info!(
                "Agent monitoring: notify script at {}",
                script_path.display()
            );
            // Install hooks at user level so they work in all repos/worktrees
            if let Err(e) = services::hook_config::install_hooks_user_level(&script_path) {
                log::warn!("Agent monitoring: failed to install user-level hooks: {e}");
            }
            commands::agent::AgentMonitorConfig { script_path }
        }
        Err(e) => {
            log::warn!("Agent monitoring: failed to create notify script: {e}");
            commands::agent::AgentMonitorConfig {
                script_path: std::path::PathBuf::new(),
            }
        }
    }
}

fn setup_agent_monitoring(
    app: &tauri::App,
    event_bus: &Arc<services::event_bus::EventBus>,
    activity_store: &Arc<services::activity_store::ActivityStore>,
) {
    activity_store.start_subscriber(event_bus);
    activity_store.start_gc(ACTIVITY_GC_TTL_MS);

    let receiver_bus = Arc::clone(event_bus);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    tauri::async_runtime::spawn(async move {
        services::hook_receiver::start(receiver_bus, HOOK_RECEIVER_PORT, ready_tx).await;
    });

    tauri::async_runtime::spawn(async move {
        match ready_rx.await {
            Ok(Ok(port)) => log::info!("Agent monitoring: hook receiver ready on port {port}"),
            Ok(Err(e)) => log::warn!("Agent monitoring disabled: {e}"),
            Err(_) => log::warn!("Agent monitoring: hook receiver startup channel dropped"),
        }
    });

    services::event_bridge::start(app.handle().clone(), event_bus);
    log::info!("Agent monitoring: event bridge started");
}

/// # Panics
///
/// Panics if the Tauri application fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    let event_bus = Arc::new(services::event_bus::EventBus::new());
    let activity_store = Arc::new(services::activity_store::ActivityStore::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                ])
                .level(log::LevelFilter::Info)
                .level_for("branchdeck_lib", log::LevelFilter::Debug)
                .build(),
        )
        .manage(Mutex::new(services::terminal::TerminalService::new()))
        .manage(Arc::clone(&activity_store))
        .manage(Arc::clone(&event_bus))
        .manage(init_agent_config())
        .manage(services::task_watcher::create_watcher_state())
        .setup(move |app| {
            setup_agent_monitoring(app, &event_bus, &activity_store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Terminal
            commands::terminal::create_terminal_session,
            commands::terminal::write_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal,
            // Git
            commands::git::add_repository,
            commands::git::list_repositories,
            commands::git::remove_repository,
            commands::git::list_worktrees_cmd,
            commands::git::create_worktree_cmd,
            commands::git::remove_worktree_cmd,
            commands::git::preview_worktree_cmd,
            commands::git::get_repo_status,
            commands::git::list_branches_cmd,
            commands::git::get_branch_tracking_cmd,
            // Workspace
            commands::workspace::get_app_config,
            commands::workspace::save_app_config,
            commands::workspace::get_repo_config,
            commands::workspace::save_repo_config_cmd,
            commands::workspace::get_presets,
            commands::workspace::save_presets,
            // GitHub
            commands::github::get_pr_status,
            commands::github::check_github_available,
            // Agent monitoring
            commands::agent::get_agents,
            commands::agent::get_file_activity,
            commands::agent::list_agent_definitions,
            commands::agent::install_agent_hooks,
            commands::agent::remove_agent_hooks,
            // Task
            commands::task::create_task_cmd,
            commands::task::get_task_cmd,
            commands::task::list_tasks_cmd,
            commands::task::start_task_watcher,
            commands::task::stop_task_watcher,
            commands::task::watch_task_path,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) =
                    window.try_state::<Mutex<services::terminal::TerminalService>>()
                {
                    if let Ok(mut service) = state.lock() {
                        service.close_all_sessions();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
