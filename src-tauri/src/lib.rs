mod commands;
mod error;
mod models;
mod services;

use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

/// # Panics
///
/// Panics if the Tauri application fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
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
        .invoke_handler(tauri::generate_handler![
            commands::terminal::create_terminal_session,
            commands::terminal::write_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal,
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
            commands::workspace::get_app_config,
            commands::workspace::save_app_config,
            commands::workspace::get_repo_config,
            commands::workspace::save_repo_config_cmd,
            commands::workspace::get_presets,
            commands::workspace::save_presets,
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
