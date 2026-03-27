use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use log::{error, info, warn};
use rand::Rng;
use std::path::{Path, PathBuf};
use subtle::ConstantTimeEq;

use crate::error::ProblemDetails;
use crate::state::AppState;

/// Length of generated tokens in bytes (rendered as 64 hex characters).
const TOKEN_BYTES: usize = 32;

/// Returns the path to the auth token file: `~/.config/branchdeck/auth.token`.
fn token_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("branchdeck").join("auth.token"))
}

/// Generate a cryptographically random 32-byte hex token.
#[must_use]
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// Save a token to `~/.config/branchdeck/auth.token` with 0o600 permissions.
///
/// # Errors
///
/// Returns an error if the config directory cannot be determined or file I/O fails.
pub fn save_token(token: &str) -> Result<(), String> {
    let path = token_path().ok_or("Cannot determine config directory")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;
    }

    std::fs::write(&path, token)
        .map_err(|e| format!("Failed to write token file: {e}"))?;

    set_file_permissions(&path)?;

    info!("Token saved to {}", path.display());
    Ok(())
}

/// Set file permissions to 0o600 (owner read/write only).
#[cfg(unix)]
fn set_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Failed to set permissions on {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// Load the stored token from `~/.config/branchdeck/auth.token`.
///
/// # Errors
///
/// Returns an error if the file cannot be read or the config directory is unknown.
pub fn load_token() -> Result<Option<String>, String> {
    let path = match token_path() {
        Some(p) => p,
        None => return Ok(None),
    };

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim().to_string();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            error!("Failed to read token file {}: {e}", path.display());
            Err(format!("Failed to read token file: {e}"))
        }
    }
}

/// Query parameter for token-based auth (used by SSE/WebSocket clients).
#[derive(serde::Deserialize, Default)]
pub struct TokenQuery {
    pub token: Option<String>,
}

/// Axum middleware that enforces bearer token authentication.
///
/// Checks `Authorization: Bearer <token>` header first, then falls back to
/// `?token=<token>` query parameter (for SSE/WebSocket where headers cannot be set).
///
/// Skipped entirely when `require_auth` is false on the `AppState`.
pub async fn auth_middleware(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
    request: Request,
    next: Next,
) -> Response {
    if !state.require_auth {
        return next.run(request).await;
    }

    let expected_token = match &state.auth_token {
        Some(t) => t,
        None => {
            error!("Auth required but no token configured");
            return unauthorized_response("Server misconfigured: no auth token");
        }
    };

    let provided_token = extract_token(&request, &query);

    match provided_token {
        Some(token) if constant_time_eq(token.as_bytes(), expected_token.as_bytes()) => {
            next.run(request).await
        }
        Some(_) => {
            warn!("Authentication failed: invalid token");
            unauthorized_response("Invalid authentication token")
        }
        None => {
            unauthorized_response("Authentication required")
        }
    }
}

/// Extract the bearer token from the request header or query parameter.
fn extract_token<'a>(request: &'a Request, query: &'a TokenQuery) -> Option<&'a str> {
    // Try Authorization header first
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(value) = auth_header.to_str() {
            if let Some(token) = value.strip_prefix("Bearer ") {
                return Some(token.trim());
            }
        }
    }

    // Fall back to query parameter (for SSE/WebSocket)
    query.token.as_deref()
}

/// Constant-time comparison to prevent timing attacks.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// Build a 401 Unauthorized response in RFC 7807 format.
fn unauthorized_response(detail: &str) -> Response {
    let problem = ProblemDetails {
        problem_type: "https://branchdeck.dev/problems/unauthorized".to_string(),
        title: "Unauthorized".to_string(),
        status: 401,
        detail: Some(detail.to_string()),
    };

    (
        StatusCode::UNAUTHORIZED,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/problem+json",
        )],
        axum::Json(problem),
    )
        .into_response()
}
