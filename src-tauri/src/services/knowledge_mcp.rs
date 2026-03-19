//! TCP endpoint for the MCP sidecar to query `KnowledgeService`.
//!
//! The MCP sidecar (Node.js stdio server) connects here via HTTP POST
//! to forward `query_knowledge` and `remember_this` tool calls.

#[cfg(feature = "knowledge")]
use crate::models::knowledge::KnowledgeType;
#[cfg(feature = "knowledge")]
use crate::services::knowledge::KnowledgeService;
#[cfg(feature = "knowledge")]
use log::{debug, error, info, warn};
#[cfg(feature = "knowledge")]
use std::sync::Arc;
#[cfg(feature = "knowledge")]
use std::time::Duration;
#[cfg(feature = "knowledge")]
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "knowledge")]
use tokio::net::TcpListener;
#[cfg(feature = "knowledge")]
use tokio::sync::oneshot;

#[cfg(feature = "knowledge")]
const MAX_PAYLOAD_BYTES: usize = 65_536;
#[cfg(feature = "knowledge")]
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Start the knowledge MCP TCP endpoint.
///
/// Binds to `127.0.0.1:0` (dynamic port). The actual port is sent back
/// via `ready_tx` so the caller can configure `settings.json`.
#[cfg(feature = "knowledge")]
pub async fn start(
    knowledge: Arc<KnowledgeService>,
    ready_tx: oneshot::Sender<Result<u16, String>>,
) {
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(l) => {
            let port = l.local_addr().map(|a| a.port()).unwrap_or(0);
            info!("Knowledge MCP endpoint listening on 127.0.0.1:{port}");
            let _ = ready_tx.send(Ok(port));
            l
        }
        Err(e) => {
            error!("Failed to bind knowledge MCP endpoint: {e}");
            let _ = ready_tx.send(Err(format!("bind failed: {e}")));
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("Knowledge MCP accepted connection from {addr}");
                let ks = Arc::clone(&knowledge);
                tokio::spawn(async move {
                    match tokio::time::timeout(CONNECTION_TIMEOUT, handle_connection(stream, &ks))
                        .await
                    {
                        Ok(Err(e)) => {
                            debug!("Knowledge MCP connection from {addr} error: {e}");
                        }
                        Err(_) => {
                            warn!("Knowledge MCP connection from {addr} timed out");
                        }
                        Ok(Ok(())) => {}
                    }
                });
            }
            Err(e) => {
                error!("Knowledge MCP accept error: {e}");
            }
        }
    }
}

#[cfg(feature = "knowledge")]
async fn handle_connection(
    stream: tokio::net::TcpStream,
    knowledge: &KnowledgeService,
) -> Result<(), String> {
    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let head = read_request_head(&mut buf_reader).await?;

    if head.content_length > MAX_PAYLOAD_BYTES {
        return respond_json(&mut writer, 400, r#"{"error":"payload too large"}"#).await;
    }

    let mut body = vec![0u8; head.content_length];
    buf_reader
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("read body: {e}"))?;

    let body_str = std::str::from_utf8(&body).map_err(|e| format!("invalid UTF-8 in body: {e}"))?;

    let response = match head.path.as_str() {
        "/knowledge/query" => handle_query(knowledge, body_str).await,
        "/knowledge/remember" => handle_remember(knowledge, body_str).await,
        "/knowledge/health" => Ok(r#"{"status":"ok"}"#.to_string()),
        _ => Err("unknown endpoint".to_string()),
    };

    match response {
        Ok(json) => respond_json(&mut writer, 200, &json).await,
        Err(e) => {
            let err_json = serde_json::json!({"error": e}).to_string();
            respond_json(&mut writer, 400, &err_json).await
        }
    }
}

#[cfg(feature = "knowledge")]
async fn handle_query(knowledge: &KnowledgeService, body: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct QueryReq {
        query: String,
        #[serde(default = "default_top_k")]
        top_k: usize,
        #[serde(default)]
        repo_path: String,
        #[serde(default)]
        worktree_id: Option<String>,
    }
    fn default_top_k() -> usize {
        5
    }

    let req: QueryReq =
        serde_json::from_str(body).map_err(|e| format!("invalid query request: {e}"))?;

    let results = knowledge
        .query(
            &req.repo_path,
            req.worktree_id.as_deref(),
            &req.query,
            req.top_k.min(100),
        )
        .await
        .map_err(|e| format!("query failed: {e}"))?;

    serde_json::to_string(&results).map_err(|e| format!("serialize failed: {e}"))
}

#[cfg(feature = "knowledge")]
async fn handle_remember(knowledge: &KnowledgeService, body: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct RememberReq {
        content: String,
        #[serde(default)]
        repo_path: String,
        #[serde(default)]
        worktree_id: Option<String>,
    }

    let req: RememberReq =
        serde_json::from_str(body).map_err(|e| format!("invalid remember request: {e}"))?;

    if req.content.trim().is_empty() {
        return Err("content must not be empty".to_string());
    }

    let id = knowledge
        .ingest_explicit(
            &req.repo_path,
            req.worktree_id.as_deref(),
            &req.content,
            KnowledgeType::Explicit,
        )
        .await
        .map_err(|e| format!("ingest failed: {e}"))?;

    Ok(serde_json::json!({"id": id}).to_string())
}

// --- HTTP helpers (same pattern as hook_receiver) ---

#[cfg(feature = "knowledge")]
struct RequestHead {
    path: String,
    content_length: usize,
}

#[cfg(feature = "knowledge")]
async fn read_request_head(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> Result<RequestHead, String> {
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .await
        .map_err(|e| format!("read request line: {e}"))?;

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    let mut content_length: usize = 0;
    let mut headers_read: usize = 0;

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

        headers_read += 1;
        if headers_read >= 32 {
            return Err("too many headers".to_string());
        }

        let lower = trimmed.to_ascii_lowercase();
        if let Some(val) = lower.strip_prefix("content-length:") {
            if let Ok(len) = val.trim().parse::<usize>() {
                content_length = len;
            }
        }
    }

    Ok(RequestHead {
        path,
        content_length,
    })
}

#[cfg(feature = "knowledge")]
async fn respond_json(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    status: u16,
    body: &str,
) -> Result<(), String> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    writer
        .write_all(response.as_bytes())
        .await
        .map_err(|e| format!("write response: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("flush response: {e}"))
}
