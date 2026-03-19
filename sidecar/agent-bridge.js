import { query } from "@anthropic-ai/claude-agent-sdk";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";

// --- State ---

/** @type {AbortController | null} */
let activeAbort = null;
/** @type {string | null} */
let activeSessionId = null;
/** @type {string | null} */
let activeTabId = null;
/** @type {number} */
let hookReceiverPort = 13370;
/** @type {Map<string, (result: object) => void>} */
const pendingPermissions = new Map();
/** @type {ReturnType<typeof setInterval> | null} */
let heartbeatInterval = null;

const HEARTBEAT_INTERVAL_MS = 30_000;

// --- Stdout protocol (sidecar → Rust run_manager) ---

function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

// --- Hook receiver HTTP POST (sidecar → Rust hook_receiver → EventBus → frontend) ---

async function postHook(payload) {
  try {
    const body = JSON.stringify(payload);
    const res = await fetch(`http://127.0.0.1:${hookReceiverPort}/hook`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "Content-Length": String(Buffer.byteLength(body)),
        ...(activeTabId ? { "X-Branchdeck-Tab-Id": activeTabId } : {}),
      },
      body,
      signal: AbortSignal.timeout(3000),
    });
    if (!res.ok) {
      console.error(`Hook POST failed: ${res.status}`);
    }
  } catch {
    // Best-effort — don't crash if hook receiver is down
  }
}

// --- Heartbeat ---

function startHeartbeat() {
  stopHeartbeat();
  heartbeatInterval = setInterval(() => {
    send({ type: "heartbeat", session_id: activeSessionId });
  }, HEARTBEAT_INTERVAL_MS);
}

function stopHeartbeat() {
  if (heartbeatInterval) {
    clearInterval(heartbeatInterval);
    heartbeatInterval = null;
  }
}

function resetRunState() {
  stopHeartbeat();
  pendingPermissions.clear();
  activeAbort = null;
  activeSessionId = null;
}

// --- SDK Hooks (real-time tool observability) ---

function buildHooks() {
  return {
    PreToolUse: [
      {
        hooks: [
          async (input, toolUseID) => {
            // Forward to hook receiver for agent monitoring UI
            postHook({
              session_id: input.session_id,
              hook_event_name: "PreToolUse",
              tool_name: input.tool_name,
              tool_input: input.tool_input,
              tool_use_id: input.tool_use_id ?? toolUseID,
              agent_id: input.agent_id ?? null,
            });
            // Also forward to run timeline via stdout
            send({
              type: "tool_call",
              tool: input.tool_name,
              file_path: extractFilePath(input.tool_name, input.tool_input),
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    PostToolUse: [
      {
        hooks: [
          async (input, toolUseID) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "PostToolUse",
              tool_name: input.tool_name,
              tool_input: input.tool_input,
              tool_use_id: input.tool_use_id ?? toolUseID,
              agent_id: input.agent_id ?? null,
            });
            // Send result summary to timeline
            const detail = summarizeToolResult(
              input.tool_name,
              input.tool_response,
            );
            send({
              type: "run_step",
              step: "tool_result",
              detail: `${input.tool_name}: ${detail}`,
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    PostToolUseFailure: [
      {
        hooks: [
          async (input) => {
            send({
              type: "run_step",
              step: "tool_error",
              detail: `${input.tool_name}: ${input.error?.slice(0, 300) ?? "unknown error"}`,
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    SubagentStart: [
      {
        hooks: [
          async (input) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "SubagentStart",
              agent_id: input.agent_id,
              agent_type: input.agent_type,
            });
            send({
              type: "run_step",
              step: "subagent_start",
              detail: `${input.agent_type} (${input.agent_id})`,
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    SubagentStop: [
      {
        hooks: [
          async (input) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "SubagentStop",
              agent_id: input.agent_id,
              agent_type: input.agent_type,
            });
            send({
              type: "run_step",
              step: "subagent_stop",
              detail: `${input.agent_type} (${input.agent_id})`,
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    Notification: [
      {
        hooks: [
          async (input) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "Notification",
              title: input.title ?? null,
              message: input.message ?? null,
            });
            send({
              type: "run_step",
              step: "notification",
              detail: input.title
                ? `${input.title}: ${input.message}`
                : input.message,
              session_id: activeSessionId,
            });
            return {};
          },
        ],
      },
    ],
    SessionStart: [
      {
        hooks: [
          async (input) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "SessionStart",
              model: input.model ?? null,
            });
            return {};
          },
        ],
      },
    ],
    Stop: [
      {
        hooks: [
          async (input) => {
            postHook({
              session_id: input.session_id,
              hook_event_name: "Stop",
            });
            return {};
          },
        ],
      },
    ],
  };
}

// --- Helper: extract file path from tool input ---

function extractFilePath(toolName, toolInput) {
  if (!toolInput || typeof toolInput !== "object") return null;
  if (["Read", "Write", "Edit", "MultiEdit"].includes(toolName)) {
    return toolInput.file_path ?? null;
  }
  if (["Glob", "Grep"].includes(toolName)) {
    return toolInput.path ?? null;
  }
  return null;
}

// --- Helper: summarize tool result for timeline ---

function summarizeToolResult(toolName, toolResponse) {
  if (toolResponse == null) return "done";
  const str = typeof toolResponse === "string"
    ? toolResponse
    : JSON.stringify(toolResponse);
  if (str.length <= 150) return str;
  return `${str.slice(0, 147)}...`;
}

// --- SDK message dispatch (conversation-level events) ---

function dispatchMessage(message) {
  switch (message.type) {
    case "system": {
      if (message.subtype === "init" && message.session_id) {
        activeSessionId = message.session_id;
        send({
          type: "session_started",
          session_id: message.session_id,
        });
        startHeartbeat();
      }
      break;
    }

    case "assistant": {
      const content = message.message?.content ?? message.content;
      if (content) {
        for (const block of content) {
          if (block.type === "text" && block.text) {
            send({
              type: "assistant_text",
              text: block.text,
              session_id: activeSessionId,
            });
          }
          // tool_use blocks are handled by PreToolUse hook, but
          // also send from here as backup for tools that bypass hooks
          if (block.type === "tool_use") {
            send({
              type: "run_step",
              step: "tool_requested",
              detail: `${block.name}${block.input?.file_path ? ` ${block.input.file_path}` : ""}`,
              session_id: activeSessionId,
            });
          }
        }
      }
      break;
    }

    case "tool_progress": {
      send({
        type: "run_step",
        step: "tool_progress",
        detail: `${message.tool_name} (${Math.round(message.elapsed_time_seconds)}s)`,
        session_id: activeSessionId,
      });
      break;
    }

    case "result": {
      const costUsd = message.total_cost_usd ?? 0;
      if (message.subtype === "success" || message.subtype === "end_turn") {
        send({
          type: "run_complete",
          status: "succeeded",
          cost_usd: costUsd,
          session_id: activeSessionId,
        });
      } else {
        send({
          type: "run_error",
          status: "failed",
          error:
            message.errors?.join("; ") ?? message.subtype ?? "unknown error",
          cost_usd: costUsd,
          session_id: activeSessionId,
        });
      }
      resetRunState();
      break;
    }

    case "stream_event":
    case "auth_status":
      break;

    default:
      break;
  }
}

// --- Session runner ---

async function runSession(request, resumeSessionId = null) {
  if (activeAbort) {
    send({
      type: "run_error",
      status: "failed",
      error: "A run is already active",
      session_id: activeSessionId,
    });
    return;
  }

  // Store config from launch request
  if (request.hook_port) hookReceiverPort = request.hook_port;
  if (request.tab_id) activeTabId = request.tab_id;

  let taskContent;
  try {
    taskContent = readFileSync(request.task_path, "utf-8");
  } catch (err) {
    send({
      type: "run_error",
      status: "failed",
      error: `Failed to read task file: ${err.message}`,
      session_id: null,
    });
    return;
  }

  activeAbort = new AbortController();

  const options = {
    cwd: request.worktree,
    abortController: activeAbort,
    permissionMode: "default",
    hooks: buildHooks(),
    canUseTool: async (toolName, input, callbackOptions) => {
      const toolUseId = callbackOptions.toolUseID;
      send({
        type: "permission_request",
        tool: toolName,
        command: input?.command ?? null,
        tool_use_id: toolUseId,
        session_id: activeSessionId,
      });
      return new Promise((resolve) => {
        pendingPermissions.set(toolUseId, resolve);
      });
    },
  };

  if (resumeSessionId) options.resume = resumeSessionId;
  if (request.options?.max_turns) options.maxTurns = request.options.max_turns;
  if (request.options?.max_budget_usd)
    options.maxBudgetUsd = request.options.max_budget_usd;

  try {
    const conversation = query({ prompt: taskContent, options });
    for await (const message of conversation) {
      if (message?.type) dispatchMessage(message);
    }
  } catch (err) {
    const errorMsg =
      err.name === "AbortError" ? "Run cancelled" : err.message;
    const status = err.name === "AbortError" ? "cancelled" : "failed";
    send({
      type: "run_error",
      status,
      error: errorMsg,
      session_id: activeSessionId,
    });
    resetRunState();
  }
}

// --- Stdin request handler ---

const rl = createInterface({ input: process.stdin, terminal: false });

rl.on("line", (line) => {
  const trimmed = line.trim();
  if (!trimmed) return;

  let request;
  try {
    request = JSON.parse(trimmed);
  } catch (err) {
    console.error(`Failed to parse stdin JSON: ${err.message}`);
    return;
  }

  switch (request.type) {
    case "launch_run":
      runSession(request, null).catch((err) => {
        console.error(`Unhandled error in launch_run: ${err.message}`);
        send({
          type: "run_error",
          status: "failed",
          error: `Internal bridge error: ${err.message}`,
          session_id: activeSessionId,
        });
        resetRunState();
      });
      break;

    case "resume_run":
      runSession(request, request.session_id).catch((err) => {
        console.error(`Unhandled error in resume_run: ${err.message}`);
        send({
          type: "run_error",
          status: "failed",
          error: `Internal bridge error: ${err.message}`,
          session_id: activeSessionId,
        });
        resetRunState();
      });
      break;

    case "cancel_run":
      if (activeAbort) {
        console.error("Cancelling active run");
        activeAbort.abort();
      } else {
        send({
          type: "run_error",
          status: "failed",
          error: "No active run to cancel",
          session_id: null,
        });
      }
      break;

    case "permission_response": {
      const resolve = pendingPermissions.get(request.tool_use_id);
      if (resolve) {
        pendingPermissions.delete(request.tool_use_id);
        if (request.decision === "approve") {
          resolve({ behavior: "allow" });
        } else {
          resolve({
            behavior: "deny",
            message: request.reason ?? "Denied by user",
          });
        }
      } else {
        console.error(
          `No pending permission for tool_use_id: ${request.tool_use_id}`,
        );
      }
      break;
    }

    default:
      console.error(`Unknown request type: ${request.type}`);
      break;
  }
});

rl.on("close", () => {
  console.error("stdin closed, shutting down sidecar");
  if (activeAbort) activeAbort.abort();
  process.exit(0);
});

for (const signal of ["SIGTERM", "SIGINT"]) {
  process.on(signal, () => {
    console.error(`Received ${signal}, shutting down`);
    if (activeAbort) activeAbort.abort();
    process.exit(0);
  });
}

console.error("Branchdeck agent bridge started (SDK v0.2.x)");
