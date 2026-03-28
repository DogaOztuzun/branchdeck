//! Update safety service.
//!
//! Tracks pending application updates and ensures they are never applied
//! while workflows are active. Updates are queued and only applied on the
//! next clean restart after all active workflows complete.
//!
//! Architecture:
//! - Pure functions for: state transitions, safety checks
//! - `UpdateState` tracks: pending version, whether workflows are active,
//!   whether the update is ready to apply

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Update state model
// ---------------------------------------------------------------------------

/// Status of a pending update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateStatus {
    /// No update available.
    None,
    /// An update has been downloaded and is waiting for workflows to finish.
    PendingWorkflowCompletion,
    /// All workflows are idle — update is ready to apply on next restart.
    ReadyToApply,
}

/// Tracks the state of a pending auto-update.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateState {
    /// Current update status.
    pub status: UpdateStatus,
    /// Version string of the pending update (e.g., "0.3.0").
    pub pending_version: Option<String>,
    /// ISO 8601 timestamp when the update was detected.
    pub detected_at: Option<String>,
    /// Whether workflows are currently active.
    pub workflows_active: bool,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            status: UpdateStatus::None,
            pending_version: None,
            detected_at: None,
            workflows_active: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure state transitions
// ---------------------------------------------------------------------------

/// Register a new available update. If workflows are active, it goes to
/// `PendingWorkflowCompletion`. If idle, it goes straight to `ReadyToApply`.
///
/// Returns the new state.
#[must_use]
pub fn register_update(
    _current: &UpdateState,
    version: &str,
    detected_at: &str,
    workflows_active: bool,
) -> UpdateState {
    let status = if workflows_active {
        info!("Update {version} available but workflows are active — queuing until completion");
        UpdateStatus::PendingWorkflowCompletion
    } else {
        info!("Update {version} available — ready to apply on next restart");
        UpdateStatus::ReadyToApply
    };

    UpdateState {
        status,
        pending_version: Some(version.to_string()),
        detected_at: Some(detected_at.to_string()),
        workflows_active,
    }
}

/// Called when workflows transition from active to idle. If an update is
/// pending, it transitions to `ReadyToApply`.
///
/// Returns the new state.
#[must_use]
pub fn on_workflows_idle(current: &UpdateState) -> UpdateState {
    let mut next = current.clone();
    next.workflows_active = false;

    if current.status == UpdateStatus::PendingWorkflowCompletion {
        info!(
            "All workflows completed — update {} is now ready to apply on next restart",
            current.pending_version.as_deref().unwrap_or("unknown")
        );
        next.status = UpdateStatus::ReadyToApply;
    } else {
        debug!("Workflows idle, no pending update");
    }

    next
}

/// Called when workflows become active. Prevents any pending update from
/// being applied.
///
/// Returns the new state.
#[must_use]
pub fn on_workflows_active(current: &UpdateState) -> UpdateState {
    let mut next = current.clone();
    next.workflows_active = true;

    // If the update was ready to apply but a new workflow started before
    // restart, downgrade back to pending
    if current.status == UpdateStatus::ReadyToApply {
        warn!(
            "Workflow started while update {} was ready — deferring again",
            current.pending_version.as_deref().unwrap_or("unknown")
        );
        next.status = UpdateStatus::PendingWorkflowCompletion;
    }

    next
}

/// Check whether it is safe to apply the update right now.
///
/// An update can only be applied when:
/// 1. There IS a pending update
/// 2. No workflows are active
/// 3. Status is `ReadyToApply`
#[must_use]
pub fn can_apply_update(state: &UpdateState) -> bool {
    state.status == UpdateStatus::ReadyToApply
        && !state.workflows_active
        && state.pending_version.is_some()
}

/// Clear the update state after it has been successfully applied.
#[must_use]
pub fn clear_update(current: &UpdateState) -> UpdateState {
    if current.pending_version.is_some() {
        info!(
            "Update {} applied successfully — clearing state",
            current.pending_version.as_deref().unwrap_or("unknown")
        );
    }
    UpdateState::default()
}

/// Build a user-visible status summary of the update state.
#[must_use]
pub fn status_summary(state: &UpdateState) -> UpdateStatusSummary {
    UpdateStatusSummary {
        has_update: state.pending_version.is_some(),
        status: state.status,
        version: state.pending_version.clone(),
        message: match state.status {
            UpdateStatus::None => "System is up to date".to_string(),
            UpdateStatus::PendingWorkflowCompletion => format!(
                "Update {} available — waiting for active workflows to complete",
                state.pending_version.as_deref().unwrap_or("unknown")
            ),
            UpdateStatus::ReadyToApply => format!(
                "Update {} ready — will apply on next restart",
                state.pending_version.as_deref().unwrap_or("unknown")
            ),
        },
    }
}

/// User-visible update status summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UpdateStatusSummary {
    /// Whether an update is available at all.
    pub has_update: bool,
    /// Current status.
    pub status: UpdateStatus,
    /// Version string if available.
    pub version: Option<String>,
    /// Human-readable status message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn register_update_when_idle_goes_to_ready() {
        let state = UpdateState::default();
        let next = register_update(&state, "0.3.0", "2026-03-27T00:00:00Z", false);

        assert_eq!(next.status, UpdateStatus::ReadyToApply);
        assert_eq!(next.pending_version.as_deref(), Some("0.3.0"));
        assert!(!next.workflows_active);
    }

    #[test]
    fn register_update_when_active_goes_to_pending() {
        let state = UpdateState::default();
        let next = register_update(&state, "0.3.0", "2026-03-27T00:00:00Z", true);

        assert_eq!(next.status, UpdateStatus::PendingWorkflowCompletion);
        assert_eq!(next.pending_version.as_deref(), Some("0.3.0"));
        assert!(next.workflows_active);
    }

    #[test]
    fn workflows_idle_transitions_pending_to_ready() {
        let state = UpdateState {
            status: UpdateStatus::PendingWorkflowCompletion,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: true,
        };

        let next = on_workflows_idle(&state);
        assert_eq!(next.status, UpdateStatus::ReadyToApply);
        assert!(!next.workflows_active);
    }

    #[test]
    fn workflows_idle_no_op_when_no_update() {
        let state = UpdateState::default();
        let next = on_workflows_idle(&state);
        assert_eq!(next.status, UpdateStatus::None);
    }

    #[test]
    fn workflows_active_downgrades_ready_to_pending() {
        let state = UpdateState {
            status: UpdateStatus::ReadyToApply,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: false,
        };

        let next = on_workflows_active(&state);
        assert_eq!(next.status, UpdateStatus::PendingWorkflowCompletion);
        assert!(next.workflows_active);
    }

    #[test]
    fn can_apply_only_when_ready_and_idle() {
        // Ready + idle = can apply
        let ready_idle = UpdateState {
            status: UpdateStatus::ReadyToApply,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: false,
        };
        assert!(can_apply_update(&ready_idle));

        // Ready but active = cannot apply
        let ready_active = UpdateState {
            status: UpdateStatus::ReadyToApply,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: true,
        };
        assert!(!can_apply_update(&ready_active));

        // Pending = cannot apply
        let pending = UpdateState {
            status: UpdateStatus::PendingWorkflowCompletion,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: true,
        };
        assert!(!can_apply_update(&pending));

        // No update = cannot apply
        assert!(!can_apply_update(&UpdateState::default()));
    }

    #[test]
    fn clear_update_resets_to_default() {
        let state = UpdateState {
            status: UpdateStatus::ReadyToApply,
            pending_version: Some("0.3.0".to_string()),
            detected_at: Some("2026-03-27T00:00:00Z".to_string()),
            workflows_active: false,
        };

        let next = clear_update(&state);
        assert_eq!(next.status, UpdateStatus::None);
        assert!(next.pending_version.is_none());
        assert!(!next.workflows_active);
    }

    #[test]
    fn status_summary_messages() {
        let none = status_summary(&UpdateState::default());
        assert!(!none.has_update);
        assert!(none.message.contains("up to date"));

        let pending = status_summary(&UpdateState {
            status: UpdateStatus::PendingWorkflowCompletion,
            pending_version: Some("0.3.0".to_string()),
            detected_at: None,
            workflows_active: true,
        });
        assert!(pending.has_update);
        assert!(pending.message.contains("waiting"));
        assert!(pending.message.contains("0.3.0"));

        let ready = status_summary(&UpdateState {
            status: UpdateStatus::ReadyToApply,
            pending_version: Some("0.3.0".to_string()),
            detected_at: None,
            workflows_active: false,
        });
        assert!(ready.has_update);
        assert!(ready.message.contains("next restart"));
        assert!(ready.message.contains("0.3.0"));
    }
}
