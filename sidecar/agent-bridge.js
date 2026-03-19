import { query } from "@anthropic-ai/claude-agent-sdk";
import { readFileSync } from "node:fs";
import { createInterface } from "node:readline";

/** @type {AbortController | null} */
let activeAbort = null;

/** @type {string | null} */
let activeSessionId = null;

/** @type {((result: { behavior: string, updatedInput?: Record<string, unknown>, message?: string }) => void) | null} */
let pendingPermissionResolve = null;

/** @type {ReturnType<typeof setInterval> | null} */
let heartbeatInterval = null;

const HEARTBEAT_INTERVAL_MS = 30_000;

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

/**
 * Write a JSON message to stdout (newline-delimited).
 * @param {Record<string, unknown>} msg
 */
function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}

/**
 * Dispatch a single SDK message to the Rust backend.
 *
 * SDK message types (v0.2.x):
 *   system        — init, compact_boundary, status, hook_response
 *   assistant     — complete assistant turn with content blocks
 *   user          — tool results flowing back (synthetic)
 *   result        — terminal: success or error_*
 *   stream_event  — partial streaming (only with includePartialMessages)
 *   tool_progress — tool execution elapsed time
 *   auth_status   — authentication state
 *
 * @param {object} message
 */
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
          if (block.type === "tool_use") {
            send({
              type: "tool_call",
              tool: block.name,
              file_path: block.input?.file_path ?? null,
              session_id: activeSessionId,
            });

            if (
              ["Edit", "Write", "MultiEdit"].includes(block.name) &&
              block.input?.file_path
            ) {
              send({
                type: "run_step",
                step: "files_changed",
                detail: block.input.file_path,
                session_id: activeSessionId,
              });
            }
          }
        }
      }
      break;
    }

    case "user": {
      const content = message.message?.content;
      if (content) {
        for (const block of content) {
          if (block.type === "tool_result" && block.tool_use_id) {
            send({
              type: "run_step",
              step: "tool_result",
              detail: block.is_error
                ? `Error: ${String(block.content).slice(0, 200)}`
                : `Completed (${block.tool_use_id.slice(0, 8)})`,
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
          error: message.errors?.join("; ") ?? message.subtype ?? "unknown error",
          cost_usd: costUsd,
          session_id: activeSessionId,
        });
      }

      resetRunState();
      break;
    }

    case "stream_event": {
      // Partial streaming messages — currently not forwarded to keep
      // the protocol simple. Could be used for live token streaming.
      break;
    }

    case "auth_status": {
      if (message.error) {
        console.error(`Auth error: ${message.error}`);
      }
      break;
    }

    default: {
      console.error(
        `Unhandled message type: ${message.type}`,
        JSON.stringify(message).slice(0, 200),
      );
      break;
    }
  }
}

function resetRunState() {
  stopHeartbeat();
  pendingPermissionResolve = null;
  activeAbort = null;
  activeSessionId = null;
}

/**
 * Shared session runner for both launch and resume flows.
 * @param {object} request
 * @param {string | null} resumeSessionId
 */
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

  /** @type {import("@anthropic-ai/claude-agent-sdk").Options} */
  const options = {
    cwd: request.worktree,
    abortController: activeAbort,
    permissionMode: "default",
    canUseTool: async (toolName, input, callbackOptions) => {
      // Use the SDK-provided toolUseID
      const toolUseId = callbackOptions.toolUseID;
      send({
        type: "permission_request",
        tool: toolName,
        command: input?.command ?? null,
        tool_use_id: toolUseId,
        session_id: activeSessionId,
      });
      return new Promise((resolve) => {
        pendingPermissionResolve = resolve;
      });
    },
  };

  if (resumeSessionId) {
    options.resume = resumeSessionId;
  }
  if (request.options?.max_turns) {
    options.maxTurns = request.options.max_turns;
  }
  if (request.options?.max_budget_usd) {
    options.maxBudgetUsd = request.options.max_budget_usd;
  }

  try {
    const conversation = query({ prompt: taskContent, options });

    for await (const message of conversation) {
      if (!message || !message.type) {
        continue;
      }
      dispatchMessage(message);
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

    case "permission_response":
      if (pendingPermissionResolve) {
        if (request.decision === "approve") {
          pendingPermissionResolve({ behavior: "allow" });
        } else {
          pendingPermissionResolve({
            behavior: "deny",
            message: request.reason ?? "Denied by user",
          });
        }
        pendingPermissionResolve = null;
      } else {
        console.error(
          "Received permission_response but no pending permission",
        );
      }
      break;

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
