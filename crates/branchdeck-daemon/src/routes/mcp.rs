use axum::extract::State;
use axum::response::{IntoResponse, Json, Response};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::state::AppState;

/// JSON-RPC 2.0 request envelope.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC 2.0 success response.
#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    result: Value,
}

/// JSON-RPC 2.0 error response.
#[derive(Serialize)]
struct JsonRpcErrorResponse {
    jsonrpc: &'static str,
    id: Value,
    error: JsonRpcError,
}

/// JSON-RPC 2.0 error object.
#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP tool definition.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolDef {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
}

/// Build the static list of MCP tool definitions.
fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "branchdeck_list_workflows",
            description: "List all registered workflows with their trigger types and outcome counts",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDef {
            name: "branchdeck_trigger_workflow",
            description: "Trigger a workflow run with a task path and optional worktree path",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_path": {
                        "type": "string",
                        "description": "Path to the task file"
                    },
                    "worktree_path": {
                        "type": "string",
                        "description": "Optional worktree path to run in"
                    }
                },
                "required": ["task_path"]
            }),
        },
        ToolDef {
            name: "branchdeck_list_runs",
            description: "List all workflow runs with their status and metadata",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDef {
            name: "branchdeck_get_run",
            description: "Get details of a specific workflow run by session ID",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Run session ID"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "branchdeck_cancel_run",
            description: "Cancel an active workflow run by session ID",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Run session ID to cancel"
                    }
                },
                "required": ["id"]
            }),
        },
        ToolDef {
            name: "branchdeck_get_sat_scores",
            description: "Get the latest SAT satisfaction score summary including aggregate score, scenario count, and findings",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        ToolDef {
            name: "branchdeck_create_worktree",
            description: "Create a new git worktree with an optional branch name and base branch",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the worktree directory"
                    },
                    "branch": {
                        "type": "string",
                        "description": "Optional branch name (defaults to worktree name)"
                    },
                    "base_branch": {
                        "type": "string",
                        "description": "Optional base branch to create from (defaults to HEAD)"
                    }
                },
                "required": ["name"]
            }),
        },
        ToolDef {
            name: "branchdeck_system_status",
            description: "Get system health status including service name, version, PID, and workspace root",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
    ]
}

/// MCP server info returned during initialization.
fn server_info() -> Value {
    json!({
        "protocolVersion": "2025-03-26",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "branchdeck",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn rpc_success(id: Value, result: Value) -> Response {
    Json(JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result,
    })
    .into_response()
}

fn rpc_error(id: Value, code: i64, message: String, data: Option<Value>) -> Response {
    Json(JsonRpcErrorResponse {
        jsonrpc: "2.0",
        id,
        error: JsonRpcError {
            code,
            message,
            data,
        },
    })
    .into_response()
}

/// JSON-RPC error codes.
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;

/// `POST /mcp` — MCP-over-HTTP endpoint using JSON-RPC 2.0.
///
/// Handles MCP protocol methods: `initialize`, `tools/list`, `tools/call`.
#[utoipa::path(
    post,
    path = "/mcp",
    request_body = Value,
    responses(
        (status = 200, description = "JSON-RPC 2.0 response")
    ),
    tag = "mcp"
)]
pub async fn mcp_handler(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let req: JsonRpcRequest = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => {
            error!("MCP: invalid JSON-RPC request: {e}");
            return rpc_error(
                Value::Null,
                INVALID_REQUEST,
                format!("Invalid JSON-RPC request: {e}"),
                None,
            );
        }
    };

    if req.jsonrpc != "2.0" {
        return rpc_error(
            req.id.unwrap_or(Value::Null),
            INVALID_REQUEST,
            "jsonrpc must be \"2.0\"".to_string(),
            None,
        );
    }

    let id = req.id.unwrap_or(Value::Null);

    debug!("MCP: method={}", req.method);

    match req.method.as_str() {
        "initialize" => {
            info!("MCP: client initialized");
            rpc_success(id, server_info())
        }
        "tools/list" => {
            debug!("MCP: listing tools");
            rpc_success(id, json!({ "tools": tool_definitions() }))
        }
        "tools/call" => dispatch_tool_call(id, req.params, &state),
        _ => rpc_error(
            id,
            METHOD_NOT_FOUND,
            format!("Unknown method: {}", req.method),
            None,
        ),
    }
}

/// Dispatch a `tools/call` request to the appropriate tool handler.
fn dispatch_tool_call(id: Value, params: Option<Value>, state: &AppState) -> Response {
    let params = match params {
        Some(p) => p,
        None => {
            return rpc_error(
                id,
                INVALID_PARAMS,
                "tools/call requires params with name and arguments".to_string(),
                None,
            );
        }
    };

    let tool_name = match params.get("name").and_then(Value::as_str) {
        Some(n) => n,
        None => {
            return rpc_error(
                id,
                INVALID_PARAMS,
                "params.name is required".to_string(),
                None,
            );
        }
    };

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    debug!("MCP: tool_call name={tool_name}");

    let result = match tool_name {
        "branchdeck_list_workflows" => tool_list_workflows(state),
        "branchdeck_trigger_workflow" => tool_trigger_workflow(&args),
        "branchdeck_list_runs" => tool_list_runs(),
        "branchdeck_get_run" => tool_get_run(&args),
        "branchdeck_cancel_run" => tool_cancel_run(&args),
        "branchdeck_get_sat_scores" => tool_get_sat_scores(state),
        "branchdeck_create_worktree" => tool_create_worktree(state, &args),
        "branchdeck_system_status" => tool_system_status(state),
        _ => {
            return rpc_error(
                id,
                METHOD_NOT_FOUND,
                format!("Unknown tool: {tool_name}"),
                None,
            );
        }
    };

    match result {
        Ok(content) => rpc_success(
            id,
            json!({
                "content": [{ "type": "text", "text": content.to_string() }]
            }),
        ),
        Err(err_msg) => rpc_success(
            id,
            json!({
                "content": [{ "type": "text", "text": err_msg }],
                "isError": true
            }),
        ),
    }
}

// --- Tool implementations: thin wrappers over core services ---

fn tool_list_workflows(state: &AppState) -> Result<Value, String> {
    let summaries: Vec<Value> = state
        .workflow_registry
        .list_workflows()
        .iter()
        .map(|w| {
            json!({
                "name": w.config.name,
                "description": w.config.description,
                "triggerKind": w.config.tracker.kind.to_string(),
                "outcomeCount": w.config.outcomes.len()
            })
        })
        .collect();
    Ok(json!(summaries))
}

fn tool_trigger_workflow(_args: &Value) -> Result<Value, String> {
    // RunManager not yet wired (stories 8.1-8.4)
    Err("Not implemented: RunManager not yet wired. Requires stories 8.1-8.4.".to_string())
}

fn tool_list_runs() -> Result<Value, String> {
    // RunManager not yet wired — return empty list (not an error, just no runs)
    Ok(json!([]))
}

fn tool_get_run(args: &Value) -> Result<Value, String> {
    let _id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "id is required".to_string())?;

    // RunManager not yet wired (stories 8.1-8.4)
    Err("Not implemented: RunManager not yet wired. Requires stories 8.1-8.4.".to_string())
}

fn tool_cancel_run(args: &Value) -> Result<Value, String> {
    let _id = args
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "id is required".to_string())?;

    // RunManager not yet wired (stories 8.1-8.4)
    Err("Not implemented: RunManager not yet wired. Requires stories 8.1-8.4.".to_string())
}

fn tool_get_sat_scores(state: &AppState) -> Result<Value, String> {
    match branchdeck_core::services::sat_score::load_latest_scores(&state.workspace_root) {
        Some(scores) => Ok(json!({
            "aggregateScore": scores.aggregate_score,
            "scenarioCount": scores.scenario_count,
            "findingCount": scores.finding_count,
            "runId": scores.run_id
        })),
        None => Ok(json!({
            "aggregateScore": null,
            "scenarioCount": 0,
            "findingCount": 0,
            "runId": null
        })),
    }
}

fn tool_create_worktree(state: &AppState, args: &Value) -> Result<Value, String> {
    let name = args
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "name is required".to_string())?;

    // Validate worktree name: allowlist of safe characters only
    if name.is_empty() || name.len() > 255 {
        return Err("Invalid worktree name: must be 1-255 characters".to_string());
    }
    if name == "." || name == ".." {
        return Err("Invalid worktree name: '.' and '..' are reserved".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(
            "Invalid worktree name: must contain only ASCII alphanumeric, dash, underscore, or dot"
                .to_string(),
        );
    }

    let branch = args.get("branch").and_then(Value::as_str);
    let base_branch = args.get("base_branch").and_then(Value::as_str);

    let wt = branchdeck_core::services::git::create_worktree(
        &state.workspace_root,
        name,
        branch,
        base_branch,
    )
    .map_err(|e| format!("Failed to create worktree: {e}"))?;

    info!("MCP: created worktree {name:?} at {}", wt.path.display());
    Ok(json!({
        "name": wt.name,
        "path": wt.path,
        "branch": wt.branch,
        "isMain": wt.is_main
    }))
}

fn tool_system_status(state: &AppState) -> Result<Value, String> {
    Ok(json!({
        "service": "branchdeck-daemon",
        "version": env!("CARGO_PKG_VERSION"),
        "pid": std::process::id(),
        "workspaceRoot": state.workspace_root.display().to_string()
    }))
}
