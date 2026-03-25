mod commands;
mod tauri_emitter;

use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

// Re-export core modules so commands can use `crate::models`, `crate::services`, `crate::error`
pub use branchdeck_core::error;
pub use branchdeck_core::models;
pub use branchdeck_core::services;
pub use branchdeck_core::traits;

pub const HOOK_RECEIVER_PORT: u16 = 13_370;
const ACTIVITY_GC_TTL_MS: u64 = 300_000; // 5 minutes
const STALE_CHECK_INTERVAL_SECS: u64 = 30;

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

/// Scan all worktrees for stale `run.json` files from a previous session.
fn recover_stale_runs(emitter: &Arc<dyn traits::EventEmitter>) {
    let config = match services::config::load_global_config() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Recovery: failed to load global config, skipping: {e}");
            return;
        }
    };

    if config.repos.is_empty() {
        log::debug!("Recovery: no repos configured, nothing to scan");
        return;
    }

    let mut worktree_paths: Vec<String> = Vec::new();
    for repo_path in &config.repos {
        match services::git::list_worktrees(std::path::Path::new(repo_path)) {
            Ok(worktrees) => {
                for wt in worktrees {
                    worktree_paths.push(wt.path.display().to_string());
                }
            }
            Err(e) => {
                log::error!("Recovery: failed to list worktrees for {repo_path}: {e}");
            }
        }
    }

    let stale_runs = services::run_state::scan_all_run_states(&worktree_paths);
    if stale_runs.is_empty() {
        log::debug!("Recovery: no stale run states found");
        return;
    }

    log::info!("Recovery: found {} stale run state(s)", stale_runs.len());

    for run_info in stale_runs {
        match run_info.status {
            models::run::RunStatus::Starting
            | models::run::RunStatus::Running
            | models::run::RunStatus::Blocked => {
                log::info!(
                    "Recovery: marking orphaned run as failed for task {}",
                    run_info.task_path
                );

                services::task::update_task_status(
                    &run_info.task_path,
                    models::task::TaskStatus::Failed,
                );

                emit_task_updated(emitter, &run_info.task_path);

                let mut failed_run = run_info.clone();
                failed_run.status = models::run::RunStatus::Failed;
                if let Err(e) = traits::emit(emitter.as_ref(), "run:status_changed", &failed_run) {
                    log::error!("Recovery: failed to emit run:status_changed: {e}");
                }

                services::run_state::delete_run_state(&run_info.task_path);
            }
            models::run::RunStatus::Failed | models::run::RunStatus::Cancelled => {
                log::debug!(
                    "Recovery: keeping run state for resumable task {}",
                    run_info.task_path
                );
            }
            models::run::RunStatus::Succeeded | models::run::RunStatus::Created => {
                log::debug!(
                    "Recovery: cleaning up terminal run state for task {}",
                    run_info.task_path
                );
                services::run_state::delete_run_state(&run_info.task_path);
            }
        }
    }
}

fn emit_task_updated(emitter: &Arc<dyn traits::EventEmitter>, task_path: &str) {
    let wt_path = std::path::Path::new(task_path)
        .parent()
        .and_then(std::path::Path::parent);

    if let Some(wt) = wt_path {
        match services::task::get_task(&wt.display().to_string()) {
            Ok(task_info) => {
                if let Err(e) = traits::emit(emitter.as_ref(), "task:updated", &task_info) {
                    log::error!("Recovery: failed to emit task:updated: {e}");
                }
            }
            Err(e) => {
                log::warn!("Recovery: failed to read task for event emission: {e}");
            }
        }
    }
}

fn start_stale_checker(run_state: services::run_manager::RunManagerState) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(STALE_CHECK_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let mut manager = run_state.lock().await;
            manager.check_stale().await;
        }
    });
    log::info!("Stale run checker started (interval: {STALE_CHECK_INTERVAL_SECS}s)");
}

fn setup_agent_monitoring(
    emitter: &Arc<dyn traits::EventEmitter>,
    event_bus: &Arc<services::event_bus::EventBus>,
    activity_store: &Arc<services::activity_store::ActivityStore>,
) {
    activity_store.start_subscriber(event_bus);
    activity_store.start_gc(ACTIVITY_GC_TTL_MS);

    let receiver_bus = Arc::clone(event_bus);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        services::hook_receiver::start(receiver_bus, HOOK_RECEIVER_PORT, ready_tx).await;
    });

    tokio::spawn(async move {
        match ready_rx.await {
            Ok(Ok(port)) => log::info!("Agent monitoring: hook receiver ready on port {port}"),
            Ok(Err(e)) => log::warn!("Agent monitoring disabled: {e}"),
            Err(_) => log::warn!("Agent monitoring: hook receiver startup channel dropped"),
        }
    });

    services::event_bridge::start(Arc::clone(emitter), event_bus);
    log::info!("Agent monitoring: event bridge started");
}

/// # Panics
///
/// Panics if the Tauri application fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used, clippy::too_many_lines)]
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
                .level_for("branchdeck_core", log::LevelFilter::Debug)
                .build(),
        )
        .manage(Mutex::new(services::terminal::TerminalService::new()))
        .manage(Arc::clone(&activity_store))
        .manage(Arc::clone(&event_bus))
        .manage(init_agent_config())
        .manage(services::task_watcher::create_watcher_state())
        .setup(move |app| {
            // Create the EventEmitter from the Tauri AppHandle
            let emitter: Arc<dyn traits::EventEmitter> =
                Arc::new(tauri_emitter::TauriEmitter::new(app.handle().clone()));

            let dev_path = || {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("CARGO_MANIFEST_DIR has parent")
                    .join("sidecar/agent-bridge.js")
            };
            let sidecar_path = app.path().resource_dir().map_or_else(
                |_| dev_path(),
                |dir| {
                    let flat = dir.join("agent-bridge.js");
                    if flat.exists() {
                        flat
                    } else {
                        dir.join("sidecar/agent-bridge.js")
                    }
                },
            );

            let sidecar_path = if sidecar_path.exists() {
                sidecar_path
            } else {
                dev_path()
            };

            log::info!("Sidecar path resolved to: {}", sidecar_path.display());
            let sidecar_dir = sidecar_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .to_path_buf();
            app.manage(services::run_manager::create_run_manager_state(
                sidecar_path,
                Arc::clone(&event_bus),
                Arc::clone(&emitter),
                HOOK_RECEIVER_PORT,
            ));
            app.manage(Arc::clone(&event_bus));

            // Store emitter in managed state for commands that need it
            app.manage(Arc::clone(&emitter));

            // Orchestrator state
            let orchestrator_state = services::orchestrator::create_orchestrator_state(
                models::orchestrator::OrchestratorConfig::default(),
            );

            let repo_paths = services::config::load_global_config()
                .map(|c| c.repos)
                .unwrap_or_default();
            {
                let mut orch = tauri::async_runtime::block_on(orchestrator_state.lock());
                for repo_path in &repo_paths {
                    if let Ok((owner, repo)) =
                        services::github::resolve_owner_repo(std::path::Path::new(repo_path))
                    {
                        orch.repo_paths
                            .insert(format!("{owner}/{repo}"), repo_path.clone());
                    }
                }
            }

            app.manage(orchestrator_state.clone());

            // PR poller shared state
            let discovered_prs: services::pr_poller::DiscoveredPrsState =
                std::sync::Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
            app.manage(discovered_prs.clone());

            recover_stale_runs(&emitter);

            // All background services use tokio::spawn internally, which requires
            // a tokio runtime context. Tauri's setup closure runs synchronously
            // before the runtime is entered, so we launch them via
            // tauri::async_runtime::spawn to bridge into the runtime.
            let rm_state: services::run_manager::RunManagerState = app
                .state::<services::run_manager::RunManagerState>()
                .inner()
                .clone();
            {
                let emitter = Arc::clone(&emitter);
                let event_bus = Arc::clone(&event_bus);
                let activity_store = Arc::clone(&activity_store);
                let stale_rm = Arc::clone(&rm_state);
                let orch_rm = Arc::clone(&rm_state);
                tauri::async_runtime::spawn(async move {
                    setup_agent_monitoring(&emitter, &event_bus, &activity_store);
                    start_stale_checker(stale_rm);
                    services::orchestrator::start_orchestrator(
                        orchestrator_state,
                        Arc::clone(&event_bus),
                        orch_rm,
                        Arc::clone(&emitter),
                    );
                    services::issue_poller::start_issue_poller(
                        Arc::clone(&event_bus),
                        repo_paths.clone(),
                    );
                    services::pr_poller::start_pr_poller(
                        event_bus,
                        repo_paths,
                        discovered_prs,
                        emitter,
                    );
                });
            }

            // Knowledge service initialization
            #[cfg(feature = "knowledge")]
            {
                match services::config::config_dir() {
                    Ok(config_dir) => {
                        match services::knowledge::KnowledgeService::new(&config_dir) {
                            Ok(knowledge_service) => {
                                let knowledge_service = Arc::new(knowledge_service);
                                app.manage(Arc::clone(&knowledge_service));

                                // Defer subscriber/MCP startup to async context
                                let ks = Arc::clone(&knowledge_service);
                                let ks_bus = Arc::clone(&event_bus);
                                let mcp_sidecar = sidecar_dir.join("knowledge-mcp.js");
                                tauri::async_runtime::spawn(async move {
                                    ks.start_subscriber(&ks_bus);
                                    #[cfg(feature = "sona")]
                                    ks.start_sona_tick();

                                    if mcp_sidecar.exists() {
                                        let mcp_ks = Arc::clone(&ks);
                                        let (mcp_tx, mcp_rx) =
                                            tokio::sync::oneshot::channel();
                                        tokio::spawn(async move {
                                            services::knowledge_mcp::start(mcp_ks, mcp_tx)
                                                .await;
                                        });
                                        tokio::spawn(async move {
                                            match mcp_rx.await {
                                                Ok(Ok(port)) => {
                                                    log::info!(
                                                        "Knowledge MCP endpoint ready on port {port}"
                                                    );
                                                    if let Err(e) =
                                                        services::hook_config::install_mcp_config(
                                                            port,
                                                            &mcp_sidecar,
                                                        )
                                                    {
                                                        log::warn!(
                                                            "Failed to install MCP config: {e}"
                                                        );
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    log::warn!(
                                                        "Knowledge MCP endpoint failed: {e}"
                                                    );
                                                }
                                                Err(_) => {
                                                    log::warn!(
                                                        "Knowledge MCP startup channel dropped"
                                                    );
                                                }
                                            }
                                        });
                                    } else {
                                        log::warn!(
                                            "knowledge-mcp.js not found at {}, skipping MCP config",
                                            mcp_sidecar.display()
                                        );
                                    }
                                });

                                log::info!("Knowledge service initialized");
                            }
                            Err(e) => {
                                log::warn!("Knowledge service initialization failed: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Could not determine config dir for knowledge service: {e}");
                    }
                }
            }

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
            commands::github::list_open_prs,
            commands::github::list_all_open_prs,
            commands::github::enrich_pr_summary,
            // Agent monitoring
            commands::agent::get_agents,
            commands::agent::get_file_activity,
            commands::agent::list_agent_definitions,
            commands::agent::install_agent_hooks,
            commands::agent::remove_agent_hooks,
            // Run
            commands::run::launch_run_cmd,
            commands::run::cancel_run_cmd,
            commands::run::get_run_status_cmd,
            commands::run::recover_runs_cmd,
            commands::run::retry_run_cmd,
            commands::run::resume_run_cmd,
            commands::run::respond_to_permission_cmd,
            commands::run::shepherd_pr_cmd,
            commands::run::batch_launch_cmd,
            commands::run::cancel_queue_cmd,
            commands::run::queue_status_cmd,
            // Orchestrator
            commands::orchestrator::relaunch_pr_cmd,
            commands::orchestrator::skip_pr_cmd,
            commands::orchestrator::get_lifecycles_cmd,
            commands::orchestrator::toggle_orchestrator_cmd,
            commands::orchestrator::orchestrator_shepherd_pr_cmd,
            commands::orchestrator::read_analysis_cmd,
            commands::orchestrator::write_approval_cmd,
            commands::orchestrator::list_discovered_prs_cmd,
            commands::orchestrator::get_running_entries_cmd,
            // Task
            commands::task::create_task_cmd,
            commands::task::get_task_cmd,
            commands::task::list_tasks_cmd,
            commands::task::start_task_watcher,
            commands::task::stop_task_watcher,
            commands::task::watch_task_path,
            // Knowledge
            #[cfg(feature = "knowledge")]
            commands::knowledge::query_knowledge,
            #[cfg(feature = "knowledge")]
            commands::knowledge::ingest_knowledge,
            #[cfg(feature = "knowledge")]
            commands::knowledge::get_knowledge_stats,
            #[cfg(feature = "knowledge")]
            commands::knowledge::forget_knowledge,
            #[cfg(feature = "sona")]
            commands::knowledge::suggest_next,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(run_state) =
                    window.try_state::<services::run_manager::RunManagerState>()
                {
                    match run_state.try_lock() {
                        Ok(mut manager) => manager.shutdown(),
                        Err(_) => {
                            log::warn!(
                                "Shutdown: RunManager lock contended, sidecar may not be killed"
                            );
                        }
                    }
                }

                #[cfg(feature = "knowledge")]
                if let Err(e) = services::hook_config::remove_mcp_config() {
                    log::warn!("Failed to remove MCP config on shutdown: {e}");
                }

                #[cfg(feature = "knowledge")]
                if let Some(knowledge) =
                    window.try_state::<Arc<services::knowledge::KnowledgeService>>()
                {
                    let ks = Arc::clone(knowledge.inner());
                    tauri::async_runtime::block_on(async move {
                        ks.close_all().await;
                    });
                    log::info!("Knowledge service closed on shutdown");
                }

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
