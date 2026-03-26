use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use branchdeck_core::models::agent::Event;
use futures::stream::Stream;
use log::{debug, error, warn};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::state::AppState;

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

/// Build an SSE envelope with typed fields per daemon-api.md spec.
fn to_sse_envelope(event: &Event, id: &str) -> serde_json::Value {
    let event_type = event_type_name(event);
    let ts = extract_timestamp(event);

    let data = serde_json::to_value(event).unwrap_or_else(|e| {
        error!("Failed to serialize SSE event data: {e}");
        serde_json::Value::Null
    });

    serde_json::json!({
        "id": id,
        "type": event_type,
        "timestamp": ts,
        "data": data
    })
}

fn extract_timestamp(event: &Event) -> u64 {
    match event {
        Event::SessionStart { ts, .. }
        | Event::ToolStart { ts, .. }
        | Event::ToolEnd { ts, .. }
        | Event::SubagentStart { ts, .. }
        | Event::SubagentStop { ts, .. }
        | Event::SessionStop { ts, .. }
        | Event::Notification { ts, .. }
        | Event::RunComplete { ts, .. }
        | Event::PrStatusChanged { ts, .. }
        | Event::IssueDetected { ts, .. }
        | Event::PrMerged { ts, .. } => *ts,
        Event::RetryDue { .. } => branchdeck_core::models::agent::now_ms(),
    }
}

static EVENT_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_event_id() -> String {
    let n = EVENT_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("evt_{n}")
}

pub async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    debug!("SSE client connected to /api/events");
    let rx = state.event_bus.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let id = next_event_id();
            let event_type = event_type_name(&event);
            let envelope = to_sse_envelope(&event, &id);
            let payload = serde_json::to_string(&envelope).unwrap_or_else(|e| {
                error!("Failed to serialize SSE envelope: {e}");
                String::new()
            });

            Some(Ok(SseEvent::default().event(event_type).id(id).data(payload)))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
            warn!("SSE subscriber lagged, missed {n} events");
            None
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}
