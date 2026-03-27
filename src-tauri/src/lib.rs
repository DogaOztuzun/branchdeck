//! Branchdeck Desktop — Tauri thin shell.
//!
//! Auto-launches the daemon, connects via HTTP/SSE, provides native chrome.
//! No business logic lives here.

mod daemon;

use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_log::{Target, TargetKind};

/// # Panics
///
/// Panics if the Tauri application fails to initialize.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    // Parse --stop-with-desktop from CLI args
    let stop_with_desktop = std::env::args().any(|a| a == "--stop-with-desktop");
    let port = resolve_port();

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
        .setup(move |app| {
            // Manage DaemonState immediately so window close handler always finds it,
            // even if the user closes the window before connect_or_spawn completes.
            app.manage(Mutex::new(daemon::DaemonState {
                child: None,
                stop_with_desktop,
            }));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let workspace = std::env::current_dir()
                    .ok()
                    .map(|p| p.to_string_lossy().to_string());
                let connection = daemon::connect_or_spawn(port, workspace.as_deref()).await;
                match connection {
                    daemon::DaemonConnection::Connected(health) => {
                        log::info!(
                            "Desktop connected to daemon v{} (pid={})",
                            health.version,
                            health.pid
                        );
                    }
                    daemon::DaemonConnection::Spawned { child, health } => {
                        log::info!(
                            "Desktop spawned daemon v{} (pid={})",
                            health.version,
                            health.pid
                        );
                        if let Some(state) = handle.try_state::<Mutex<daemon::DaemonState>>() {
                            if let Ok(mut daemon) = state.lock() {
                                daemon.child = Some(child);
                            }
                        }
                    }
                    daemon::DaemonConnection::Failed(reason) => {
                        log::error!("Failed to connect to daemon: {reason}");
                        // Emit an error event so the frontend can display it
                        if let Err(e) = handle.emit("daemon:connection_failed", &reason) {
                            log::error!("Failed to emit connection error: {e}");
                        }
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if let Some(state) = window.try_state::<Mutex<daemon::DaemonState>>() {
                    if let Ok(mut daemon) = state.lock() {
                        daemon.shutdown();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Resolve the daemon port from environment or default.
fn resolve_port() -> u16 {
    std::env::var("BRANCHDECK_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(daemon::DEFAULT_PORT)
}
