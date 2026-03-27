//! Daemon lifecycle management for the desktop thin client.
//!
//! Handles health checks, spawning the daemon as a child process,
//! and waiting for it to become ready.

use log::{debug, error, info, warn};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

/// Default port for the daemon.
pub const DEFAULT_PORT: u16 = 13_371;

/// Maximum time to wait for the daemon to become healthy after spawning.
const HEALTH_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

/// Interval between health check polls during startup.
const HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Expected service name in health response.
const EXPECTED_SERVICE: &str = "branchdeck-daemon";

/// Health response from the daemon's `GET /api/health` endpoint.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
    pub pid: u32,
    pub workspace_root: String,
}

/// Result of the daemon connection attempt.
pub enum DaemonConnection {
    /// Connected to an already-running daemon.
    Connected(HealthResponse),
    /// Spawned a new daemon child process and connected.
    Spawned {
        child: Child,
        health: HealthResponse,
    },
    /// Failed to connect or spawn.
    Failed(String),
}

/// Check the daemon health endpoint.
///
/// Returns `Ok(health)` if the daemon is running and healthy,
/// `Err` if the connection was refused or the response is invalid.
pub async fn check_daemon_health(port: u16) -> Result<HealthResponse, String> {
    let url = format!("http://127.0.0.1:{port}/api/health");
    debug!("Checking daemon health at {url}");

    let response = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Health check returned {}", response.status()));
    }

    let health: HealthResponse = response
        .json()
        .await
        .map_err(|e| format!("Invalid health response: {e}"))?;

    Ok(health)
}

/// Validate that the health response matches our expectations.
fn validate_health(health: &HealthResponse, workspace: Option<&str>) -> Result<(), String> {
    if health.service != EXPECTED_SERVICE {
        return Err(format!(
            "Port is in use by a different service: '{}'. \
             Configure a different port with BRANCHDECK_PORT or --port.",
            health.service
        ));
    }

    if let Some(expected_workspace) = workspace {
        if health.workspace_root != expected_workspace {
            return Err(format!(
                "Daemon on this port serves a different workspace: '{}'. \
                 Expected: '{expected_workspace}'. \
                 Configure a different port with BRANCHDECK_PORT or --port.",
                health.workspace_root
            ));
        }
    }

    Ok(())
}

/// Spawn the daemon as a child process.
///
/// Looks for the `branchdeck-daemon` binary next to the desktop binary,
/// then falls back to `$PATH`.
fn spawn_daemon(port: u16, workspace: Option<&str>) -> Result<Child, String> {
    let daemon_bin = find_daemon_binary();
    info!(
        "Spawning daemon: {} serve --port {port}",
        daemon_bin.display()
    );

    let mut cmd = std::process::Command::new(&daemon_bin);
    cmd.arg("serve").arg("--port").arg(port.to_string());

    if let Some(ws) = workspace {
        cmd.arg("--workspace").arg(ws);
    }

    // Discard stdout (daemon uses stderr via env_logger).
    // Inherit stderr so daemon logs are visible during development.
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit());

    cmd.spawn().map_err(|e| {
        error!(
            "Failed to spawn daemon binary '{}': {e}",
            daemon_bin.display()
        );
        format!(
            "Failed to start daemon: {e}. \
             Ensure 'branchdeck-daemon' is installed or available in PATH."
        )
    })
}

/// Find the daemon binary.
///
/// Search order:
/// 1. Same directory as the current executable
/// 2. `$PATH` lookup
fn find_daemon_binary() -> PathBuf {
    // Try sibling of current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("branchdeck-daemon");
            if sibling.exists() {
                return sibling;
            }
        }
    }

    // Fall back to PATH
    PathBuf::from("branchdeck-daemon")
}

/// Wait for the daemon to become healthy, polling at regular intervals.
async fn wait_for_health(port: u16) -> Result<HealthResponse, String> {
    let deadline = tokio::time::Instant::now() + HEALTH_WAIT_TIMEOUT;

    loop {
        match check_daemon_health(port).await {
            Ok(health) => return Ok(health),
            Err(e) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "Daemon did not become healthy within {}s: {e}",
                        HEALTH_WAIT_TIMEOUT.as_secs()
                    ));
                }
                debug!("Daemon not ready yet: {e}");
                tokio::time::sleep(HEALTH_POLL_INTERVAL).await;
            }
        }
    }
}

/// Connect to the daemon, spawning it if necessary.
///
/// Flow:
/// 1. Check health on configured port
/// 2. If healthy + correct service: connect
/// 3. If connection refused: spawn daemon, wait for health, connect
/// 4. If healthy + wrong service/workspace: return error
pub async fn connect_or_spawn(port: u16, workspace: Option<&str>) -> DaemonConnection {
    // Step 1: Try connecting to an existing daemon
    match check_daemon_health(port).await {
        Ok(health) => {
            // Step 2: Validate the running daemon
            if let Err(msg) = validate_health(&health, workspace) {
                return DaemonConnection::Failed(msg);
            }
            info!(
                "Connected to existing daemon (pid={}, version={}, workspace={})",
                health.pid, health.version, health.workspace_root
            );
            return DaemonConnection::Connected(health);
        }
        Err(e) => {
            debug!("No daemon running on port {port}: {e}");
        }
    }

    // Step 3: Spawn daemon
    let child = match spawn_daemon(port, workspace) {
        Ok(c) => c,
        Err(msg) => return DaemonConnection::Failed(msg),
    };

    // Wait for the daemon to become healthy
    match wait_for_health(port).await {
        Ok(health) => {
            // Validate health after spawn to ensure we connected to our daemon
            if let Err(msg) = validate_health(&health, workspace) {
                return DaemonConnection::Failed(msg);
            }
            info!(
                "Spawned daemon (pid={}, version={}, workspace={})",
                health.pid, health.version, health.workspace_root
            );
            DaemonConnection::Spawned { child, health }
        }
        Err(msg) => {
            warn!("Spawned daemon but it failed to become healthy: {msg}");
            DaemonConnection::Failed(msg)
        }
    }
}

/// Managed state for the daemon child process (if we spawned it).
pub struct DaemonState {
    /// Child process handle, if we spawned the daemon.
    pub child: Option<Child>,
    /// Whether to kill the daemon on desktop close.
    pub stop_with_desktop: bool,
}

impl DaemonState {
    /// Shut down the daemon child process if `stop_with_desktop` is enabled.
    pub fn shutdown(&mut self) {
        if !self.stop_with_desktop {
            info!(
                "Desktop closing — daemon will keep running (pid={:?})",
                self.child.as_ref().map(Child::id)
            );
            return;
        }

        if let Some(ref mut child) = self.child {
            let pid = child.id();
            info!("Desktop closing — sending SIGTERM to daemon (pid={pid})");

            // Send SIGTERM for graceful shutdown via Command
            #[cfg(unix)]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .status();
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
                return;
            }

            // Wait up to 3 seconds for graceful exit
            for _ in 0..30 {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        info!("Daemon exited gracefully (pid={pid})");
                        return;
                    }
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(e) => {
                        warn!("Error waiting for daemon: {e}");
                        break;
                    }
                }
            }

            // Force kill if still running
            warn!("Daemon did not exit after SIGTERM — sending SIGKILL (pid={pid})");
            if let Err(e) = child.kill() {
                warn!("Failed to kill daemon process: {e}");
            }
        }
    }
}
