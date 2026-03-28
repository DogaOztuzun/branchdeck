use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use branchdeck_core::models::agent::{now_ms, Event};
use futures::stream::Stream;
use log::{debug, error, warn};
use serde::Serialize;
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use utoipa::ToSchema;

use crate::state::AppState;

/// SSE event envelope per daemon-api.md spec.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub struct SseEnvelope {
    /// Unique event ID in `evt_<ulid>` format.
    pub id: String,
    /// Event type in `namespace:snake_case_action` format.
    #[serde(rename = "type")]
    pub event_type: String,
    /// Epoch millisecond timestamp.
    pub timestamp: u64,
    /// Run/session ID when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Event-specific payload.
    pub data: serde_json::Value,
}

/// Map an `Event` variant to its SSE event type string.
fn event_type_name(event: &Event) -> &'static str {
    match event {
        Event::SessionStart { .. } => "agent:session_start",
        Event::ToolStart { .. } => "agent:tool_start",
        Event::ToolEnd { .. } => "agent:tool_end",
        Event::SubagentStart { .. } => "agent:subagent_start",
        Event::SubagentStop { .. } => "agent:subagent_stop",
        Event::SessionStop { .. } => "agent:session_stop",
        Event::Notification { .. } => "agent:notification",
        Event::RunComplete { .. } => "run:complete",
        Event::PrStatusChanged { .. } => "workflow:pr_status_changed",
        Event::RetryDue { .. } => "workflow:retry_due",
        Event::IssueDetected { .. } => "workflow:issue_detected",
        Event::PrMerged { .. } => "workflow:pr_merged",
    }
}

/// Extract the run/session ID from event variants that carry one.
fn extract_run_id(event: &Event) -> Option<String> {
    match event {
        Event::SessionStart { session_id, .. }
        | Event::ToolStart { session_id, .. }
        | Event::ToolEnd { session_id, .. }
        | Event::SubagentStart { session_id, .. }
        | Event::SubagentStop { session_id, .. }
        | Event::SessionStop { session_id, .. }
        | Event::Notification { session_id, .. }
        | Event::RunComplete { session_id, .. } => Some(session_id.clone()),
        Event::PrStatusChanged { .. }
        | Event::RetryDue { .. }
        | Event::IssueDetected { .. }
        | Event::PrMerged { .. } => None,
    }
}

/// Build an SSE envelope with typed fields per daemon-api.md spec.
fn to_sse_envelope(event: &Event, id: &str) -> SseEnvelope {
    let event_type = event_type_name(event).to_owned();
    let ts = event.timestamp();
    // RetryDue has no timestamp — use current time as fallback
    let timestamp = if ts == 0 { now_ms() } else { ts };
    let run_id = extract_run_id(event);

    let data = serde_json::to_value(event).unwrap_or_else(|e| {
        error!("Failed to serialize SSE event data: {e}");
        serde_json::Value::Null
    });

    SseEnvelope {
        id: id.to_owned(),
        event_type,
        timestamp,
        run_id,
        data,
    }
}

fn next_event_id() -> String {
    format!("evt_{}", ulid::Ulid::new())
}

/// Stream daemon events as Server-Sent Events.
///
/// All daemon events are streamed as JSON typed envelopes using the format:
/// `{ "id": "evt_<ulid>", "type": "namespace:action", "timestamp": ..., "run_id": ..., "data": {} }`
///
/// Event namespaces: `run:`, `agent:`, `workflow:`, `sat:`, `system:`
#[utoipa::path(
    get,
    path = "/api/events",
    responses(
        (status = 200, description = "SSE event stream", content_type = "text/event-stream")
    ),
    tag = "events"
)]
pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    debug!("SSE client connected to /api/events");
    let rx = state.event_bus.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let id = next_event_id();
            let event_type = event_type_name(&event).to_owned();
            let envelope = to_sse_envelope(&event, &id);
            let payload = serde_json::to_string(&envelope).unwrap_or_else(|e| {
                error!("Failed to serialize SSE envelope: {e}");
                String::new()
            });

            Some(Ok(SseEvent::default()
                .event(event_type)
                .id(id)
                .data(payload)))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
            warn!("SSE subscriber lagged, missed {n} events");
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
