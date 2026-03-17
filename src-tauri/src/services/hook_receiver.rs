use crate::models::agent::{now_ms, Event, HookPayload};
use crate::services::event_bus::EventBus;
use log::{debug, error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

const MAX_PAYLOAD_BYTES: usize = 65_536;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

const RESPONSE_200: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const RESPONSE_400: &[u8] =
    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// Start the hook receiver TCP listener.
///
/// Binds to `127.0.0.1:{port}` and accepts connections.  Each connection is
/// handled in a spawned task that parses a minimal HTTP request, deserializes
/// the JSON body into [`HookPayload`], converts it to an [`Event`], and
/// publishes it on the [`EventBus`].
///
/// `ready_tx` signals whether the bind succeeded or failed so the caller can
/// react immediately without polling.
pub async fn start(
    event_bus: Arc<EventBus>,
    port: u16,
    ready_tx: oneshot::Sender<Result<(), String>>,
) {
    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => {
            info!("Hook receiver listening on 127.0.0.1:{port}");
            let _ = ready_tx.send(Ok(()));
            l
        }
        Err(e) => {
            error!("Failed to bind hook receiver on 127.0.0.1:{port}: {e}");
            let _ = ready_tx.send(Err(format!("bind failed: {e}")));
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("Hook receiver accepted connection from {addr}");
                let bus = Arc::clone(&event_bus);
                tokio::spawn(async move {
                    match tokio::time::timeout(CONNECTION_TIMEOUT, handle_connection(stream, &bus))
                        .await
                    {
                        Ok(Err(e)) => {
                            debug!("Hook connection from {addr} ended with error: {e}");
                        }
                        Err(_) => {
                            warn!("Hook connection from {addr} timed out");
                        }
                        Ok(Ok(())) => {}
                    }
                });
            }
            Err(e) => {
                error!("Hook receiver accept error: {e}");
            }
        }
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    event_bus: &EventBus,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let head = read_request_head(&mut buf_reader).await?;

    if !head.is_post_hook {
        return respond(&mut writer, RESPONSE_400).await;
    }
    if head.content_length > MAX_PAYLOAD_BYTES {
        warn!(
            "Hook payload too large: {} bytes (max {MAX_PAYLOAD_BYTES})",
            head.content_length
        );
        return respond(&mut writer, RESPONSE_400).await;
    }

    // Read body
    let mut body = vec![0u8; head.content_length];
    buf_reader
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("read body: {e}"))?;

    // Deserialize
    let payload: HookPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to parse hook payload: {e}");
            return respond(&mut writer, RESPONSE_400).await;
        }
    };

    // Tab/session IDs come from HTTP headers (injected by notify.sh)
    let tab_id = head.tab_id.unwrap_or_else(|| "unknown".to_string());
    let session_id = payload.session_id.clone();
    let ts = now_ms();

    if let Some(event) = payload_to_event(&payload, &session_id, &tab_id, ts) {
        debug!("Publishing event for session {session_id}, tab {tab_id}");
        let _ = event_bus.publish(event);
    }

    respond(&mut writer, RESPONSE_200).await
}

struct RequestHead {
    is_post_hook: bool,
    content_length: usize,
    tab_id: Option<String>,
}

async fn read_request_head(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<RequestHead, String> {
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read request line: {e}"))?;

    let is_post_hook = request_line.starts_with("POST /hook");
    let mut content_length: usize = 0;
    let mut tab_id: Option<String> = None;

    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("read header: {e}"))?;

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        let lower = trimmed.to_ascii_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        } else if lower.starts_with("x-branchdeck-tab-id:") {
            // Read original (non-lowered) value after the header name
            if let Some(val) = trimmed.get("x-branchdeck-tab-id:".len()..) {
                let v = val.trim();
                if !v.is_empty() {
                    tab_id = Some(v.to_string());
                }
            }
        }
    }

    Ok(RequestHead {
        is_post_hook,
        content_length,
        tab_id,
    })
}

async fn respond(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    response: &[u8],
) -> Result<(), String> {
    writer
        .write_all(response)
        .await
        .map_err(|e| format!("write response: {e}"))
}

fn extract_file_path(tool_name: &str, tool_input: Option<&serde_json::Value>) -> Option<String> {
    let input = tool_input?;
    let obj = input.as_object()?;

    match tool_name {
        "Read" | "Write" | "Edit" => obj
            .get("file_path")
            .and_then(serde_json::Value::as_str)
            .map(String::from),
        "Glob" | "Grep" => obj
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(String::from),
        _ => None,
    }
}

fn payload_to_event(
    payload: &HookPayload,
    session_id: &str,
    tab_id: &str,
    ts: u64,
) -> Option<Event> {
    match payload.hook_event_name.as_str() {
        "SessionStart" => Some(Event::SessionStart {
            session_id: session_id.to_string(),
            tab_id: tab_id.to_string(),
            model: payload.model.clone(),
            ts,
        }),
        "PreToolUse" => {
            let tool_name = payload.tool_name.clone().unwrap_or_default();
            let file_path = extract_file_path(&tool_name, payload.tool_input.as_ref());
            Some(Event::ToolStart {
                session_id: session_id.to_string(),
                agent_id: payload.agent_id.clone(),
                tab_id: tab_id.to_string(),
                tool_name,
                tool_use_id: payload.tool_use_id.clone().unwrap_or_default(),
                file_path,
                ts,
            })
        }
        "PostToolUse" => {
            let tool_name = payload.tool_name.clone().unwrap_or_default();
            let file_path = extract_file_path(&tool_name, payload.tool_input.as_ref());
            Some(Event::ToolEnd {
                session_id: session_id.to_string(),
                agent_id: payload.agent_id.clone(),
                tab_id: tab_id.to_string(),
                tool_name,
                tool_use_id: payload.tool_use_id.clone().unwrap_or_default(),
                file_path,
                ts,
            })
        }
        "SubagentStart" => Some(Event::SubagentStart {
            session_id: session_id.to_string(),
            agent_id: payload.agent_id.clone().unwrap_or_default(),
            agent_type: payload.agent_type.clone().unwrap_or_default(),
            tab_id: tab_id.to_string(),
            ts,
        }),
        "SubagentStop" => Some(Event::SubagentStop {
            session_id: session_id.to_string(),
            agent_id: payload.agent_id.clone().unwrap_or_default(),
            agent_type: payload.agent_type.clone().unwrap_or_default(),
            tab_id: tab_id.to_string(),
            ts,
        }),
        "Stop" => Some(Event::SessionStop {
            session_id: session_id.to_string(),
            tab_id: tab_id.to_string(),
            ts,
        }),
        "Notification" => Some(Event::Notification {
            session_id: session_id.to_string(),
            tab_id: tab_id.to_string(),
            title: payload.title.clone(),
            message: payload.message.clone().unwrap_or_default(),
            ts,
        }),
        other => {
            warn!("Unknown hook event: {other:?}, skipping");
            None
        }
    }
}
